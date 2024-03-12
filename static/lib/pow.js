import mitt from "mitt";

import { dispense, getSession } from "lib/api";

const emitter = mitt();
const worker = new Worker(new URL("/static/worker.js", import.meta.url));
const query = new URLSearchParams(document.location.search);
const method = query.get("method") ?? "pow";

export class PoW {
	constructor() {
		this.isStarted = false;
		this.working = false;
		this.providerUrl = null;
		this.address = null;
		this.hash = null;
	}

	get isUsing() {
		return method === "pow";
	}

	setWorking(working) {
		this.working = working;
	}
	setProviderUrl(providerUrl) {
		this.providerUrl = providerUrl;
	}
	setAddress(address) {
		this.address = address;
	}

	start() {
		worker.onmessage = async (event) => {
			switch (event.data.type) {
				case "hash": {
					try {
						console.log("hash", event.data);
						const hash = await this.callDispense(event.data.value);
						this.hash = hash;
						emitter.emit("done", hash);
					} catch (error) {
						emitter.emit("error", error);
					}
					break;
				}
				case "stopped":
					console.log("stopped", event.data);
					this.setWorking(false);
					emitter.emit("stop");
					break;
				default:
					console.error("Unhandled event.data", event.data);
					emitter.emit("error", event.data);
					return; // unhandled or TODO
			}
		};
		this.isStarted = true;
	}

	stop() {
		this.setWorking(false);
		worker.postMessage(null);
		emitter.emit("stop");
	}

	async toggle() {
		if (this.working) {
			this.stop();
			return;
		}

		emitter.emit("start");
		const payload = {
			address: this.address,
			captcha: "",
		};

		// if (hasCaptcha()) {
		// 	data.captcha = form["g-recaptcha-response"].value;
		// }

		try {
			const data = await getSession(payload);
			if (data.error) {
				this.stop();
				emitter.emit("error", data.error);
				throw new Error(data.error);
			}

			this.setWorking(true);
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
