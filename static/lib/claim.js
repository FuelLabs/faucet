import { dispense, getSession } from "lib/api";
import mitt from "mitt";

const emitter = mitt();
const worker = new Worker(new URL("/static/worker.js", import.meta.url));

export class Claim {
	constructor() {
		this.address = null;
		this.providerUrl = null;
		this.working = false;
	}

	get isUsing() {
		return method === "auth";
	}

	setAddress(address) {
		this.address = address;
	}

	setProviderUrl(url) {
		this.providerUrl = url;
	}

	setup() {
		worker.onmessage = async (event) => {
			switch (event.data.type) {
				case "hash": {
					try {
						const hash = await this.callDispense(event.data.value);
						emitter.emit("done", hash);
					} catch (error) {
						emitter.emit("error", error);
					}
					break;
				}
				case "stopped":
					this.working = false;
					emitter.emit("stop");
					break;
				default:
					emitter.emit("error", event.data);
					return; // unhandled or TODO
			}
		};
	}

	async withAuth() {
		const payload = {
			address: this.address,
		};

		try {
			const data = await dispense(payload, "auth");
			if (data.error) {
				emitter.emit("error", data.error);
				throw new Error(data.error);
			}
			emitter.emit("done", data);
			return data;
		} catch (error) {
			console.log(error);
			emitter.emit("error", error);
		}
	}

	stop() {
		this.working = false;
		worker.postMessage(null);
		emitter.emit("stop");
	}

	async withPow() {
		if (this.working) {
			this.stop();
			return;
		}

		emitter.emit("start");
		const payload = {
			address: this.address,
		};

		try {
			const data = await getSession(payload);
			if (data.error) {
				this.stop();
				emitter.emit("error", data.error);
				throw new Error(data.error);
			}

			this.working = true;
			worker.postMessage({
				salt: data.salt,
				difficultyLevel: data.difficulty,
			});
		} catch (error) {
			emitter.emit("error", error);
			this.stop();
		}
	}

	async callDispense(payload) {
		const data = await dispense(payload, "pow");
		if (data.error) {
			this.stop();
			emitter.emit("error", data.error);
			throw new Error(data.error);
		}
		return data;
	}

	onStart(cb) {
		emitter.on("start", cb);
		return () => emitter.off("start", cb);
	}

	onStop(cb) {
		emitter.on("stop", cb);
		return () => emitter.off("stop", cb);
	}

	onDone(cb) {
		emitter.on("done", cb);
		return () => emitter.off("done", cb);
	}

	onError(cb) {
		emitter.on("error", cb);
		return () => emitter.off("error", cb);
	}
}
