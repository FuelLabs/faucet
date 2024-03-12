import confetti from "https://esm.sh/canvas-confetti@1.6.0";
import { useComputed, useSignal } from "@preact/signals";
import { html } from "lib/html";
import { PoW } from "lib/pow";
import { useEffect } from "preact/hooks";

import { Captcha } from "components/captcha";

function AlertError({ error }) {
	if (!error) return null;
	return html`<div role="alert" class=${styles.alertError}>${error}</div>`;
}

function AlertClaimSuccess({ explorerLink }) {
	return html` <div role="alert" class=${styles.alertSuccess}>
    <h2 class="text-green-700">Test Ether sent to the wallet</h2>
    <a href=${explorerLink} class=${styles.explorerLink}
      >See on Fuel Explorer</a
    >
  </div>`;
}

function AlertPowSuccess() {
	return html` <div role="alert" class=${styles.alertPowSuccess}>
    Funds sent to the wallet
  </div>`;
}

const query = new URLSearchParams(document.location.search);
const queryAddress = query.get("address") ?? "";
const pow = new PoW();

export function FaucetForm({ providerUrl, captchaKey }) {
	const state = useSignal("idle");
	const error = useSignal(null);
	const address = useSignal(queryAddress);
	const explorerLink = useSignal(null);
	const submitText = useSignal(
		pow.isUsing ? "Start PoW" : "Give me test Ether",
	);

	const isSubmitDisabled = useComputed(
		() =>
			!address.value.length ||
			state.value === "loading" ||
			state.value === "pow:done",
	);

	const isPowDone = useComputed(() => {
		return state.value === "pow:done";
	});
	const isClaimSent = useComputed(() => {
		return explorerLink.value && state.value === "done";
	});

	async function onSubmit(e) {
		e.preventDefault();
		if (pow.isUsing) {
			await pow.toggle();
		} else {
			// todo: sned claim with auth
		}
	}

	function onInput(e) {
		address.value = e.target.value;
	}

	useEffect(() => {
		pow.setProviderUrl(providerUrl);
		pow.setAddress(address.value);
		pow.start();
		const subs = [
			pow.onStart(() => {
				submitText.value = "Stop PoW";
				state.value = "pow:working";
				error.value = null;
			}),
			pow.onStop(() => {
				submitText.value = "Start PoW";
				state.value = "pow:stopped";
			}),
			pow.onError((err) => {
				error.value = err ?? err.toString().message ?? "Unknown error";
				state.value = "pow:error";
				submitText.value = "Start PoW";
			}),
			pow.onDone(() => {
				error.value = null;
				submitText.value = "Start PoW";
				state.value = "pow:done";
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

	function getForm() {
		if (state.value.includes("done")) return null;
		return html`
      <div class=${styles.formWrapper}>
        <label for="address" class=${styles.label}>Wallet Address</label>
        <input
          type="text"
          id="address"
          name="address"
          autocomplete="off"
          minlength="63"
          placeholder="fuel100000... or 0x0000..."
          pattern="[a-z0-9]{63,66}"
          class=${styles.input}
          value=${address.value}
          onInput=${onInput}
        />
      </div>
    `;
	}

	return html`
    <div>
      <form onSubmit=${onSubmit}>
        ${getForm()}
        <p class="text-center text-gray-800 text-sm [&_span]:font-bold">
          This is a <span>Test Ether</span> faucet running on the${" "}
          <span>Test Fuel network</span>. This faucet sends fake Ether assets to
          the provided wallet address.
        </p>
        <${Captcha}
          captchaKey=${captchaKey}
          isHidden=${state.value.includes("done")}
        />
        <${AlertError} error=${error.value?.toString()} />
        <div
          class=${`text-center mt-6 ${
						state.value.includes("done") && "hidden"
					}`}
        >
          <button
            type="submit"
            class=${styles.submitButton}
            disabled=${isSubmitDisabled.value}
          >
            ${submitText.value}
          </button>
        </div>
        ${
					state.value === "pow:working" &&
					html`
          <div
            class="w-full flex items-center justify-center mt-2 text-sm text-gray-500"
          >
            <span class="loader w-4 h-4"></span>
            <span class="ms-2"
              >Please, waiting until Proof of Work get finished!</span
            >
          </div>
        `
				}
      </form>

      ${isPowDone.value && html`<${AlertPowSuccess} />`}
      ${
				isClaimSent.value &&
				html`<${AlertClaimSuccess} explorerLink=${explorerLink.value} />`
			}
    </div>
  `;
}

const styles = {
	formWrapper: "border p-4 mb-4 flex flex-col rounded-lg",
	label: "mb-2 text-md text-gray-500",
	input:
		"border border-gray-300 text-gray-900 text-sm rounded focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5",
	explorerLink: "underline underline-offset-2",
	alertError:
		"flex flex-col items-center py-2 px-4 border border-red-300 mt-6 gap-1 text-sm rounded-lg bg-red-50 text-red-800",
	alertSuccess:
		"flex flex-col items-center p-4 border border-gray-300 mt-6 gap-1 text-sm rounded-lg bg-gray-50",
	alertPowSuccess:
		"flex flex-col items-center p-4 border border-green-300 mt-6 gap-1 text-sm rounded-lg bg-green-50",
	submitButton:
		"text-black bg-[#02F58C] font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 disabled:bg-gray-300 disabled:text-gray-800 disabled:cursor-not-allowed",
	agreements:
		"flex flex-col gap-2 text-sm mt-6 py-4 border-t border-b border-gray-300 border-dashed [&_label>span]:font-bold",
};
