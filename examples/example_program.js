let uppercase = false, intentionalTimeout = false, intentionalCrash = false, interactive = false, useLists = false;

for (const arg of process.argv.slice(2)) {
	if (arg === "--uppercase") uppercase = true;
	else if (arg === "--intentional-timeout") intentionalTimeout = true;
	else if (arg === "--intentional-crash") intentionalCrash = true;
	else if (arg === "--interactive") interactive = true;
	else if (arg === "--use-lists") useLists = true;
}

// console.error({ uppercase, intentionalTimeout, intentionalCrash, useLists });

if (!interactive) {
	let value = process.argv[2];
	if (useLists) {
		value = value.split("\n").map(line => `- ${line}`).join("\n");
	}
	console.log(uppercase ? value.toUpperCase() : uppercase);
} else {
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
			if (useLists) writer.write("- ");
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
}
