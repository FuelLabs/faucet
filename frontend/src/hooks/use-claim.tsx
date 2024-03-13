import Clerk from "@clerk/clerk-js";
import { useComputed, useSignal } from "@preact/signals";
import confetti from "canvas-confetti";
import { useEffect } from "preact/hooks";

import * as api from "../lib/api";
import { Claim } from "../lib/claim";

const query = new URLSearchParams(document.location.search);
const queryAddress = query.get("address") ?? "";
const claim = new Claim();
const clerk = new Clerk(window.__CLERK_PUB_KEY__, {
	domain: "https://quick-crawdad-10.clerk.accounts.dev",
});

export function useClaim(providerUrl: string) {
	const address = useSignal<string | null>(queryAddress);
	const error = useSignal<any>(null);
	const state = useSignal<string>("loading");
	const method = useSignal<"auth" | "pow" | null>(null);
	const isDone = state.value?.includes("done");
	const isSignedIn = useSignal(false);
	const isLoading = useComputed(() => state.value === "loading");
	const isWorking = useComputed(() => state.value === "working");
	const isDisabled = useComputed(
		() => !address.value?.length || state.value === "loading" || isDone,
	);

	async function fetchSession() {
		state.value = "loading";
		await clerk.load();
		const { value, sessions } = await api.getClerkSession(clerk);
		if (value) {
			await api.validateClerkSession({ value });
			isSignedIn.value = true;
			method.value = "auth";
			state.value = "idle";
			return;
		}
		await api.removeSession();
		state.value = "idle";
		return sessions;
	}

	async function submitUsingAuth() {
		state.value = "loading";
		error.value = null;

		await clerk.load();
		const { value } = await api.getClerkSession(clerk);
		if (value) {
			await api.validateClerkSession({ value });
			await claim.withAuth();
			return;
		}

		const body = document.querySelector("#root");
		const overlay = document.createElement("div");
		overlay.id = "overlay";
		body?.appendChild(overlay);
		clerk.mountSignIn(overlay, {
			routing: "virtual",
			redirectUrl: `/?address=${address.value}`,
		});
	}

	async function submitUsingPow() {
		await claim.withPow();
	}

	async function onSubmit(e: any) {
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

	function setMethod(value: "auth" | "pow" | null) {
		return () => {
			method.value = value;
		};
	}

	function onInput(e: any) {
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
		fetchSession().then(() => {
			if (clerk.user) {
				const userBtn = document.querySelector("#clerk-user");
				clerk.mountUserButton(userBtn as any);
				return;
			}

			clerk.addListener(async (resources) => {
				if (!resources.session) {
					const res = await fetch("/api/session/remove", {
						method: "POST",
						headers: {
							"Content-Type": "application/json",
						},
					});
					await res.json();
				}
			});
		});
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
		isDone,
		onSubmit,
		onInput,
		setMethod,
		submitPowText,
		submitAuthText,
	};
}
