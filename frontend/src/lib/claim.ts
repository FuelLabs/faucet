import mitt from "mitt";
import * as api from "../lib/api";

const emitter = mitt();

export class Claim {
	address: string | null;
	providerUrl: string | null;

	constructor() {
		this.address = null;
		this.providerUrl = null;
	}

	setAddress(address: string | null) {
		this.address = address;
	}

	setProviderUrl(url: string) {
		this.providerUrl = url;
	}

	async withAuth() {
		const payload = {
			address: this.address,
		};

		try {
			const data = await api.dispense(payload, "auth");
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

	onDone(cb: () => void) {
		emitter.on("done", cb);
		return () => emitter.off("done", cb);
	}

	onError(cb: (err: any) => void) {
		emitter.on("error", cb);
		return () => emitter.off("error", cb);
	}
}
