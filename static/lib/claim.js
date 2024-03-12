import mitt from "mitt";

import { dispense } from "lib/api";

const emitter = mitt();
const query = new URLSearchParams(document.location.search);
const method = query.get("method") ?? "pow";

export class Claim {
	constructor() {
		this.address = null;
		this.providerUrl = null;
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

	async dispense() {
		const payload = {
			address: this.address,
			captcha: "",
		};

		// if (hasCaptcha()) {
		// 	data.captcha = form["g-recaptcha-response"].value;
		// }

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

	onDone(cb) {
		emitter.on("done", cb);
		return () => emitter.off("done", cb);
	}

	onError(cb) {
		emitter.on("error", cb);
		return () => emitter.off("error", cb);
	}
}
