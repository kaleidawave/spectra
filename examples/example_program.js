const args = process.argv.slice(2);
let value = args[0];
if (args.includes("--use-lists")) {
	value = value.split("\n").map(line => `- ${line}`).join("\n");
}

if (args.includes("--uppercase")) console.log(value.toUpperCase());
else console.log(value);