use cspuz_rs::graph;

pub struct ForcedLinesPayload {
    pub url: String,
    pub forced_lines: graph::BoolGridEdgesIrrefutableFacts,
}

fn parse_forced_state_grid(src: &json::JsonValue) -> Result<Vec<Vec<Option<bool>>>, &'static str> {
    if !src.is_array() {
        return Err("invalid forced line grid shape");
    }
    let mut ret = vec![];
    let mut width = None;
    for y in 0..src.len() {
        let row = &src[y];
        if !row.is_array() {
            return Err("invalid forced line grid shape");
        }
        match width {
            Some(width) if width != row.len() => return Err("invalid forced line grid shape"),
            None => width = Some(row.len()),
            _ => {}
        }
        let mut parsed_row = vec![];
        for x in 0..row.len() {
            match row[x].as_i32() {
                Some(1) => parsed_row.push(Some(true)),
                Some(0) => parsed_row.push(Some(false)),
                Some(-1) => parsed_row.push(None),
                _ => return Err("invalid forced line grid value"),
            }
        }
        ret.push(parsed_row);
    }
    Ok(ret)
}

pub fn deserialize_payload(payload: &[u8]) -> Result<ForcedLinesPayload, &'static str> {
    let payload = std::str::from_utf8(payload)
        .map_err(|_| "failed to decode forced line payload as UTF-8")?;
    let root = json::parse(payload).map_err(|_| "forced line payload JSON parsing failed")?;
    let url = root["url"]
        .as_str()
        .ok_or("forced line payload URL missing")?
        .to_owned();
    let forced_h = parse_forced_state_grid(&root["forcedH"])?;
    let forced_v = parse_forced_state_grid(&root["forcedV"])?;

    Ok(ForcedLinesPayload {
        url,
        forced_lines: graph::BoolGridEdgesIrrefutableFacts {
            horizontal: forced_h,
            vertical: forced_v,
        },
    })
}
