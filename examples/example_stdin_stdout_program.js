const arg1 = process.argv[2];
const arg2 = process.argv[3];

const uppercase = arg1 === "--uppercase";
const intentionalTimeout = arg2 === "--intentional-timeout";
const intentionalCrash = arg2 === "--intentional-crash";

console.log({ uppercase, intentionalTimeout, intentionalCrash });

const wait = (duration = 1000) => new Promise((res, _rej) => setTimeout(res, duration));

async function sendMessage(total) {
	if (total.trimEnd().endsWith("2")) {
		if (intentionalTimeout) await wait(3000);
		if (intentionalCrash) throw Error("CRASH!!!");
	}

	const bothChannels = total.includes("stderr");

	for (const line of total.split("\n")) {
		const chunk = uppercase ? line.toUpperCase() : line;
		const stream = line.endsWith("on stderr") ? Bun.stderr : Bun.stdout;

		const writer = stream.writer();
		writer.write(chunk);
		writer.write("\n");
		writer.flush();

		if (bothChannels) await wait(50);
	}
}

console.log("start");
let buffer = "";
for await (const line of console) {
	if (line == "close") break;

	if (line == "end") {
		await sendMessage(buffer);
		console.log("end");
		buffer = "";
		continue
	}

	buffer += line;
	buffer += "\n";
}

console.log("finished");
