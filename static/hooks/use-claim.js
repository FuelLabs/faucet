import confetti from "https://esm.sh/canvas-confetti@1.6.0";
import { useComputed, useSignal } from "@preact/signals";
import { useEffect } from "preact/hooks";

import { removeSession, validateSession } from "lib/api";
import { Claim } from "lib/claim";

const query = new URLSearchParams(document.location.search);
const queryAddress = query.get("address") ?? "";
const claim = new Claim();
const clerk = window.Clerk;

export function useClaim({ providerUrl }) {
	const address = useSignal(queryAddress);
	const error = useSignal(null);
	const state = useSignal("loading");
	const method = useSignal(null);
	const isDone = state.value?.includes("done");
	const isSignedIn = useSignal(false);
	const isLoading = useComputed(() => state.value === "loading");
	const isWorking = useComputed(() => state.value === "working");
	const isDisabled = useComputed(
		() => !address.value.length || state.value === "loading" || isDone,
	);

	async function fetchSession() {
		state.value = "loading";
		await clerk?.load();
		const sessions = await clerk?.user?.getSessions();
		if (sessions?.length) {
			await validateSession(sessions);
			isSignedIn.value = true;
			method.value = "auth";
			state.value = "idle";
			return;
		}
		await removeSession();
		state.value = "idle";
		return sessions;
	}

	async function submitUsingAuth() {
		state.value = "loading";
		error.value = null;
		await clerk.load();

		if (clerk.user) {
			const sessions = await clerk.user?.getSessions();
			await validateSession(sessions);
			await claim.withAuth();
			return;
		}

		const body = document.querySelector("#root");
		const overlay = document.createElement("div");
		overlay.id = "overlay";
		body.appendChild(overlay);
		clerk.mountSignIn(overlay, {
			routing: "virtual",
			redirectUrl: `/?address=${address.value}`,
		});
	}

	async function submitUsingPow() {
		await claim.withPow();
	}

	async function onSubmit(e) {
		e.preventDefault();

		if (method.value === "auth") {
			await submitUsingAuth();
			return;
		}

		if (method.value === "pow") {
			await submitUsingPow();
			return;
		}
	}

	function setMethod(value) {
		return () => {
			method.value = value;
		};
	}

	function onInput(e) {
		address.value = e.target.value;
	}

	function submitPowText() {
		if (isLoading.value) return "Loading";
		if (isWorking.value) return "Stop PoW";
		return "Claim with Pow";
	}
	function submitAuthText() {
		if (isLoading.value) return "Loading";
		return "Claim with Auth";
	}

	useEffect(() => {
		claim.setProviderUrl(providerUrl);
		claim.setAddress(address.value);
		claim.setup();

		const subs = [
			claim.onStart(() => {
				state.value = "working";
				error.value = null;
			}),
			claim.onStop(() => {
				state.value = "idle";
			}),
			claim.onError((err) => {
				error.value = err.message;
				state.value = "error";
			}),
			claim.onDone(() => {
				state.value = "done";
				error.value = null;
				confetti({
					particleCount: 100,
					spread: 70,
					origin: { y: 0.6 },
				});
			}),
		];

		return () => {
			subs.forEach((sub) => sub());
		};
	}, [providerUrl, address.value]);

	useEffect(() => {
		fetchSession();
	}, []);

	return {
		address: address.value,
		error: error.value,
		state: state.value,
		method: method.value,
		isSignedIn: isSignedIn.value,
		isDisabled: isDisabled.value,
		isLoading: isLoading.value,
		isWorking: isWorking.value,
		isDone: isDone.value,
		onSubmit,
		onInput,
		setMethod,
		submitPowText,
		submitAuthText,
	};
}
