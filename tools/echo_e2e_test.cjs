// E2E test for the echo solver in the wasm backend.
const path = require("path");
const ModuleFactory = require(path.resolve(
	__dirname,
	"..",
	"..",
	"pzprjs/dist/wasm/cspuz_solver_backend.js"
));
const Module = ModuleFactory.default || ModuleFactory;

const WASM_PATH = path.resolve(
	__dirname,
	"..",
	"..",
	"pzprjs/dist/wasm/cspuz_solver_backend.wasm"
);

function solve(mod, url) {
	const buf = mod._prepare_input_buffer(url.length);
	// write utf8
	const bytes = Buffer.from(url, "utf8");
	mod.HEAPU8.set(bytes, buf);
	const outptr = mod._solve_problem(buf, url.length);
	// first 4 bytes: little-endian length
	const len =
		mod.HEAPU8[outptr] |
		(mod.HEAPU8[outptr + 1] << 8) |
		(mod.HEAPU8[outptr + 2] << 16) |
		(mod.HEAPU8[outptr + 3] << 24);
	const jsonBytes = [];
	for (let i = 0; i < len; i++) {
		jsonBytes.push(mod.HEAPU8[outptr + 4 + i]);
	}
	return Buffer.from(jsonBytes).toString("utf8");
}

Module({ locateFile: function(f) {
	return f.endsWith(".wasm") ? WASM_PATH : f;
} }).then(mod => {
	const url =
		"https://puzz.link/p?echo/5/5/g1012030340i1202012010g1012020120i3403012010g";
	const res = solve(mod, url);
	console.log(res);
	const parsed = JSON.parse(res);
	if (parsed.status !== "ok") {
		console.error("FAIL: status", parsed.status);
		process.exit(1);
	}
	const d = parsed.description;
	console.log("isUnique:", d.isUnique);
	// count green fills (black cells) and black clue texts
	let fills = 0,
		clueTexts = 0;
	const data = Array.isArray(d.data) ? d.data : [d.data];
	for (const item of data) {
		const kind = typeof item.item === "string" ? item.item : item.item.kind;
		if (item.color === "green" && kind === "fill") fills++;
		else if (item.color === "black" && kind === "text") clueTexts++;
	}
	console.log("black cells:", fills, "clue items:", clueTexts);
	// Expected solution: 9 black cells; the 16 white cells all carry clues
	// (24 clue numbers in total).
	if (fills !== 9 || clueTexts !== 24) {
		console.error("FAIL: unexpected item counts");
		process.exit(1);
	}
	console.log("E2E OK");
});
