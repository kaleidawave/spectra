export class FramedMessageReader {
	#reader: ReadableStreamReader<number>;
	#buffer: Uint8Array;

	constructor(readableStream: ReadableStream) {
		this.#reader = readableStream.getReader();
		this.#buffer = new Uint8Array(0);
	}

	#appendToBuffer(chunk: Uint8Array) {
		const newBuffer = new Uint8Array(this.#buffer.length + chunk.length);
		newBuffer.set(this.#buffer);
		newBuffer.set(chunk, this.#buffer.length);
		this.#buffer = newBuffer;
	}

	#tryReadMessage() {
		if (this.#buffer.length < 2) {
			return null;
		}

		// parse 16 bit unsigned integer little-endian form 
		const messageLength = this.#buffer[0] | (this.#buffer[1] << 8);
		const totalFrameLength = 2 + messageLength;

		if (this.#buffer.length < totalFrameLength) {
			return null;
		}

		const messageData = this.#buffer.slice(2, totalFrameLength);

		// Remove consumed bytes from buffer
		this.#buffer = this.#buffer.slice(totalFrameLength);

		return messageData;
	}

	async #readMessage() {
		while (true) {
			// Try to parse a message from existing buffer
			const message = this.#tryReadMessage();
			if (message !== null) {
				return message;
			}

			const { value, done } = await this.#reader.read();

			if (done) {
				if (this.#buffer.length > 0) {
					throw new Error(`Stream ended with incomplete message (${this.#buffer.length} bytes remaining)`);
				}
				return null;
			}

			this.#appendToBuffer(value);
		}
	}

	async *[Symbol.asyncIterator]() {
		while (true) {
			const message = await this.#readMessage();
			if (message === null) break;
			yield new TextDecoder().decode(message);
		}
	}

	releaseLock() {
		this.#reader.releaseLock();
	}
}

export function writeFrame(writer: WritableStreamDefaultWriter, frame: Uint8Array) {
	// Write little-endian unsigned 16-bit integer
	{
		const buffer = new Uint8Array(2);
		buffer[0] = frame.length & 0b1111_1111;
		buffer[1] = (frame.length >> 8) & 0b1111_1111;
		writer.write(buffer);
	}
	writer.write(frame);
}
