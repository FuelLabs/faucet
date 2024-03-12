const worker = new Worker(new URL("/static/worker.js", import.meta.url));
import mitt from "mitt";

const emitter = mitt();

export class PoW {
	constructor() {
		this.isStarted = false;
		this.working = false;
		this.providerUrl = null;
		this.address = null;
		this.hash = null;
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
		if (this.isStarted) return;
		if (!this.address) {
			emitter.emit("error", "Address not set");
			return;
		}
		if (!this.providerUrl) {
			emitter.emit("error", "Provider URL not set");
			return;
		}

		worker.onmessage = async (event) => {
			switch (event.data.type) {
				case "finish":
					emitter.emit("finish", event.data);
					break;
				case "hash": {
					const hash = await this.callDispense(event.data.value);
					emitter.emit("hash", hash);
					this.hash = hash;
					this.stop();
					break;
				}
				case "stopped":
					this.setWorking(false);
					emitter.emit("stop");
					break;
				default:
					emitter.emit("error", event.data);
					console.error("Unhandled event.data", event.data);
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
			const response = await fetch("/session", {
				method: "POST",
				headers: {
					Accept: "application/json",
					"Content-Type": "application/json",
				},
				body: JSON.stringify(payload),
			});

			const data = await response.json();
			if (data.error) {
				this.stop();
				emitter.emit("error", data.error);
				return;
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
		const res = await fetch("/dispense", {
			method: "POST",
			headers: {
				Accept: "application/json",
				"Content-Type": "application/json",
			},
			body: JSON.stringify(payload),
		});

		const data = await res.json();
		if (data.error) {
			emitter.emit("error", data.error);
			return;
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

	onFinish(cb) {
		emitter.on("finish", cb);
		return () => emitter.off("finish", cb);
	}

	onError(cb) {
		emitter.on("error", cb);
		return () => emitter.off("error", cb);
	}
}
