import { useClaim } from "../hooks/use-claim";

function AlertError({ error }: { error: string }) {
	if (!error) return null;
	return (
		<div role="alert" class={styles.alertError}>
			{error}
		</div>
	);
}

function AlertSuccess() {
	return (
		<div role="alert" class={styles.alertSuccess}>
			<h2 class="text-green-700">Funds sent to the wallet</h2>
		</div>
	);
}

function Submit({
	children,
	disabled,
	onClick,
	isHidden,
}: {
	children: any;
	disabled: boolean;
	onClick: any;
	isHidden: boolean;
}) {
	if (isHidden) return null;
	return (
		<button
			type="submit"
			class={styles.submitButton}
			disabled={disabled}
			onClick={onClick}
		>
			{children}
		</button>
	);
}

export function FaucetForm({ providerUrl }: { providerUrl: string }) {
	const {
		address,
		error,
		method,
		isSignedIn,
		isDisabled,
		isLoading,
		isDone,
		onSubmit,
		onInput,
		setMethod,
		submitAuthText,
	} = useClaim(providerUrl);

	const onSubmitAuth = setMethod("auth");

	function getForm() {
		if (isDone) return null;
		return (
			<div class={styles.formWrapper}>
				<label for="address" class={styles.label}>
					Wallet Address
				</label>
				<input
					type="text"
					id="address"
					name="address"
					autocomplete="off"
					minLength={63}
					placeholder="fuel100000... or 0x0000..."
					pattern="[a-z0-9]{63,66}"
					class={styles.input}
					value={address || ""}
					onInput={onInput}
				/>
			</div>
		);
	}

	return (
		<div>
			<form onSubmit={onSubmit}>
				<input type="hidden" name="method" value={method ?? ""} />
				{getForm()}
				<p class="text-center text-gray-800 text-sm [&_span]:font-bold">
					This is a <span>Test Ether</span> faucet running on the{" "}
					<span>Test Fuel network</span>. This faucet sends fake Ether assets to
					the provided wallet address.
				</p>
				<AlertError error={error?.toString()} />
				{isLoading && (
					<div class="flex items-center justify-center mt-6">
						<div class="loader w-4 h-4" />
					</div>
				)}
				<div
					class={`flex items-center justify-center mt-6 ${
						(isDone || isLoading) && "hidden"
					}`}
				>
					<Submit
						disabled={isDisabled}
						onClick={onSubmit}
						isHidden={!isSignedIn}
					>
						{isLoading ? "Loading..." : "Send me test ETH"}
					</Submit>
					<Submit
						disabled={isDisabled}
						onClick={onSubmitAuth}
						isHidden={isSignedIn}
					>
						{submitAuthText()}
					</Submit>
				</div>
			</form>
			{isDone && <AlertSuccess />}
		</div>
	);
}

const styles = {
	formWrapper: "border p-4 mb-4 flex flex-col rounded-lg",
	label: "mb-2 text-md text-gray-500",
	input:
		"border border-gray-300 text-gray-900 text-sm rounded focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5",
	alertError:
		"flex flex-col items-center py-2 px-4 border border-red-300 mt-6 gap-1 text-sm rounded-lg bg-red-50 text-red-800",
	alertSuccess:
		"flex flex-col items-center p-4 border border-green-300 mt-6 gap-1 text-sm rounded-lg bg-green-50",
	submitButton:
		"text-black bg-[#02F58C] hover:bg-[#02E281] font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 disabled:bg-gray-300 disabled:text-gray-800 disabled:cursor-not-allowed",
};
