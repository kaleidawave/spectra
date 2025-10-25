import { FramedMessageReader, writeFrame } from "./index.ts"

const process = Bun.spawn(
	["target/debug/examples/out"],
	{ stdin: "pipe", stdout: "pipe", stderr: "inherit" }
);

const reader = new FramedMessageReader(process.stdout);

for await (const chunk of reader) {
	console.log(chunk)
	if (chunk === "Hello World") {
		writeFrame(process.stdin, new TextEncoder().encode("haha!!"))
	}
}
