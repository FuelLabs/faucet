import { html } from "htm/preact";
import { Component } from "preact";

import { Captcha } from "components/captcha";
import { Checkbox } from "components/checkbox";

const query = new URLSearchParams(document.location.search);
const address = query.get("address");

function AlertError({ error }) {
	if (!error) return null;
	return html`<div role="alert" class=${styles.alertError}>${error}</div>`;
}
function AlertSuccess({ explorerLink, isSent }) {
	if (isSent) {
		return html` <div role="alert" class=${styles.alertSuccess}>
      <h2 class="text-green-700">Test Ether sent to the wallet</h2>
      <a href=${explorerLink} class=${styles.explorerLink}
        >See on Fuel Explorer</a
      >
    </div>`;
	}
	return null;
}

export class FaucetForm extends Component {
	state = {
		value: address,
		formHidden: false,
		hasAgreed1: false,
		hasAgreed2: false,
		hasAgreed3: false,
		isSent: false,
		explorerLink: "#",
		error: null,
	};

	onSubmit = async (e) => {
		e.preventDefault();

		const payload = {
			address: this.state.value,
			captcha: "",
		};

		// if (this.hasCaptcha()) {
		// 	const target = e.currentTarget;
		// 	payload.captcha = target.querySelector(".g-recaptcha-response")?.value;
		// }

		try {
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
				this.setState((state) => ({
					...state,
					error: data.error,
				}));
				return;
			}

			const blockExplorer = "https://fuellabs.github.io/block-explorer-v2";
			const providerUrl = this.props.providerUrl;
			const encodedProviderUrl = encodeURIComponent(providerUrl);
			const { value: address } = this.state;
			this.setState((state) => ({
				...state,
				inSent: true,
				formHidden: true,
				explorerLink: `${blockExplorer}/address/${address}?providerUrl=${encodedProviderUrl}`,
			}));
		} catch (e) {
			console.log("error");
			this.setState((state) => ({
				...state,
				error: e.message,
			}));
		}
	};

	changeAgreement = (num) => {
		return (e) => {
			this.setState((state) => ({
				...state,
				[`hasAgreed${num}`]: e.currentTarget.checked,
			}));
		};
	};

	onInput = (e) => {
		this.setState((state) => ({
			...state,
			value: e.currentTarget.value,
		}));
	};

	hasCaptcha = () => {
		return !!document.getElementsByClassName("captcha-container")[0];
	};

	render({ captchaKey }) {
		const { formHidden } = this.state;
		return html`
      <div>
        <form onSubmit=${this.onSubmit}>
          ${this.formElement()}

          <p class="text-center text-gray-800 text-sm [&_span]:font-bold">
            This is a <span>Test Ether</span> faucet running on the${" "}
            <span>Test Fuel network</span>. This faucet sends fake Ether assets
            to the provided wallet address.
          </p>

          <${Captcha} captchaKey=${captchaKey} isHidden=${formHidden} />

          <div class=${styles.agreements}>
            <${Checkbox} id="agreement1" onChange=${this.changeAgreement(1)}>
              I acknowledge that this faucet is <span>only used for testing</span>.
            </${Checkbox}>
            <${Checkbox} id="agreement2" onChange=${this.changeAgreement(2)}>
              I acknowledge that there are <span>no incentives</span> to using this faucet.
            </${Checkbox}>
            <${Checkbox} id="agreement3" onChange=${this.changeAgreement(3)}>
              I agree not to spam this faucet, and know that I will be blocked if I do.
            </${Checkbox}>
          </div>

          <${AlertError} error=${this.state.error} />
          <div class="text-center mt-6">
            <button
              type="submit"
              class=${styles.submitButton}
              disabled=${Boolean(
								!this.state.value?.length ||
									!this.state.hasAgreed1 ||
									!this.state.hasAgreed2 ||
									!this.state.hasAgreed3,
							)}
            >
              Give me Test Ether
            </button>
          </div>
        </form>
        <${AlertSuccess}
          explorerLink=${this.state.explorerLink}
          isSent=${this.state.isSent}
        />
      </div>
    `;
	}

	formElement = () => {
		const { value, formHidden } = this.state;
		if (formHidden) return null;
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
          value=${value}
          onInput=${this.onInput}
        />
      </div>
    `;
	};
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
	submitButton:
		"text-white bg-green-700 hover:bg-green-800 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 focus:outline-none disabled:bg-gray-300 disabled:text-gray-800 disabled:cursor-not-allowed",
	agreements:
		"flex flex-col gap-2 text-sm mt-6 py-4 border-t border-b border-gray-300 border-dashed [&_label>span]:font-bold",
};
