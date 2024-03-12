import { html } from "lib/html";
import { Component, render } from "preact";

import { FaucetForm } from "components/faucet-form";
import { FuelLogo } from "components/fuel-logo";

const query = new URLSearchParams(document.location.search);
const method = query.get("method") ?? "pow";

if (method === "auth") {
	window.addEventListener("load", async () => {
		await Clerk.load();

		if (Clerk.user) {
			const userBtn = document.querySelector("#clerk-user");
			Clerk.mountUserButton(userBtn);
		}

		Clerk.addListener(async (resources) => {
			if (!resources.session) {
				const res = await fetch("/api/session/remove", {
					method: "POST",
					headers: {
						"Content-Type": "application/json",
					},
				});
				await res.json();
				window.location.reload();
			}
		});
	});
}

class App extends Component {
	render({ publicNodeUrl, captchaKey }) {
		return html`
      <div
        class="background w-[100vw] h-[100vh] flex flex-col items-center justify-center"
      >
        <div
          class="relative max-w-[550px] p-6 bg-white border border-gray-200 rounded-lg shadow"
        >
          <${FuelLogo} />
          <${FaucetForm}
            providerUrl=${publicNodeUrl}
            captchaKey=${captchaKey}
          />
        </div>
        <div class="mt-6 text-xs text-gray-400 text-center">
          Node url: ${publicNodeUrl}
        </div>
      </div>
    `;
	}
}

export default function renderFaucet(props) {
	render(html`<${App} ...${props} />`, document.querySelector("#root"));
}
