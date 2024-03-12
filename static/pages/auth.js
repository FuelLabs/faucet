import { html } from "lib/html";
import { Component, render } from "preact";

import { FuelLogo } from "components/fuel-logo";

const ICON_SIZE = 48;

class App extends Component {
	state = {
		isLoading: true,
		method: null,
	};

	componentDidMount() {
		this.loadClerk();
	}

	async loadClerk() {
		await Clerk.load();
		await this.checkAuth();
	}

	async checkAuth() {
		if (Clerk.user) {
			const sessions = await Clerk.user.getSessions();
			if (sessions.length) {
				const res = await fetch("/api/session/validate", {
					method: "POST",
					headers: {
						"Content-Type": "application/json",
					},
					body: JSON.stringify({ value: sessions[0]?.id }),
				});
				const data = await res.json();
				if (data?.id) {
					window.location.reload();
				}
			}
		}
		this.setState({ isLoading: false });
	}

	async showSignIn() {
		await this.checkAuth();
		const body = document.querySelector("#root");
		const overlay = document.createElement("div");
		overlay.id = "overlay";
		body.appendChild(overlay);
		Clerk.mountSignIn(overlay);
	}

	render() {
		if (this.state.isLoading) {
			return html`
        <div
          className="background w-[100vw] h-[100vh] flex flex-col items-center justify-center"
        >
          <${FuelLogo} />
          <div class="max-w-sm">
            <h2 class="text-2xl text-center">Loading...</h2>
          </div>
        </div>
      `;
		}
		return html`
      <div
        className="background w-[100vw] h-[100vh] flex flex-col items-center justify-center"
      >
        <div class="bg-white border rounded-lg show p-8 max-w-md">
          <${FuelLogo} />
          <h2 class="text-2xl text-center">
            Which method do you prefer for claim your tokens?
          </h2>
          <div class="flex items-center justify-center gap-4 mt-6">
            <button
              class=${styles.buttonSelected}
              onClick=${() => {
								this.setState({ method: "pow" });
								window.location.href = "/?method=pow";
							}}
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width=${ICON_SIZE}
                height=${ICON_SIZE}
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="icon icon-tabler icons-tabler-outline icon-tabler-clock-bolt"
              >
                <path stroke="none" d="M0 0h24v24H0z" fill="none" />
                <path d="M20.984 12.53a9 9 0 1 0 -7.552 8.355" />
                <path d="M12 7v5l3 3" />
                <path d="M19 16l-2 3h4l-2 3" />
              </svg>
              Proof of Work
            </button>
            <button
              class=${styles.buttonSelected}
              onClick=${async () => {
								this.setState({ method: "auth" });
								await this.showSignIn();
							}}
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width=${ICON_SIZE}
                height=${ICON_SIZE}
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="icon icon-tabler icons-tabler-outline icon-tabler-user-scan"
              >
                <path stroke="none" d="M0 0h24v24H0z" fill="none" />
                <path d="M10 9a2 2 0 1 0 4 0a2 2 0 0 0 -4 0" />
                <path d="M4 8v-2a2 2 0 0 1 2 -2h2" />
                <path d="M4 16v2a2 2 0 0 0 2 2h2" />
                <path d="M16 4h2a2 2 0 0 1 2 2v2" />
                <path d="M16 20h2a2 2 0 0 0 2 -2v-2" />
                <path d="M8 16a2 2 0 0 1 2 -2h4a2 2 0 0 1 2 2" />
              </svg>
              Social Auth
            </button>
          </div>
        </div>
      </div>
    `;
	}
}

export default function renderSignIn(props) {
	render(html`<${App} ...${props} />`, document.querySelector("#root"));
}

const styles = {
	buttonSelected:
		"w-[200px] text-gray-500 text-center p-4 rounded-xl border text-xl bg-gray-100 flex flex-col items-center justify-center gap-4 focus:outline-none focus:ring-2 focus:ring-green-200 focus:ring-offset-2 hover:text-black hover:border-green cursor-pointer transition-all duration-200 ease-in-out",
};
