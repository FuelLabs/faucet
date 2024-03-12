import { html } from "htm/preact";
import { Component, render } from "preact";

class App extends Component {
	state = {
		isLoading: true,
	};

	componentDidMount() {
		this.loadClerk();
	}

	async loadClerk() {
		await Clerk.load();

		if (Clerk.user) {
			const sessions = await Clerk.user.getSessions();
			if (sessions.length) {
				const res = await fetch("/api/validate-session", {
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
		} else {
			const body = document.querySelector("#root");
			const overlay = document.createElement("div");
			overlay.id = "overlay";
			body.appendChild(overlay);
			Clerk.mountSignIn(overlay);
			this.setState({ isLoading: false });
		}
	}

	render() {
		return html`
      <div
        className="background w-[100vw] h-[100vh] flex items-center justify-center"
      >
        ${this.state.isLoading ? html`Loading...` : ""}
      </div>
    `;
	}
}

export default function renderSignIn(props) {
	render(html`<${App} ...${props} />`, document.querySelector("#root"));
}
