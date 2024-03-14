import { Component } from "preact";

import { FaucetForm } from "./components/faucet-form";
import { FuelLogo } from "./components/fuel-logo";

export class App extends Component<{ providerUrl: string }> {
	render() {
		const { providerUrl } = this.props;
		return (
			<div class="background w-[100vw] h-[100vh] flex flex-col items-center justify-center">
				<div class="relative max-w-[550px] p-6 bg-white border border-gray-200 rounded-lg shadow">
					<FuelLogo />
					<FaucetForm providerUrl={providerUrl} />
				</div>
				<div class="mt-6 text-xs text-gray-400 text-center">
					Node url: ${providerUrl}
				</div>
			</div>
		);
	}
}
