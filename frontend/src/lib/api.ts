import Clerk from "@clerk/clerk-js";

type DispenseMethod = "auth";
export type DispenseInput = {
	salt?: string;
	nonce?: string;
	address?: string | null;
};

type DispenseResponse = {
	status: string;
	tokens: number;
	error?: string;
};

export async function dispense(payload: DispenseInput, method: DispenseMethod) {
	const response = await fetch(`/api/dispense?method=${method}`, {
		method: "POST",
		headers: {
			Accept: "application/json",
			"Content-Type": "application/json",
		},
		body: JSON.stringify(payload),
	});
	return response.json() as Promise<DispenseResponse>;
}

type RemoveSessionResponse = {
	status: string;
	error?: string;
};

export async function removeSession() {
	const response = await fetch("/api/session/remove", {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
		},
		body: JSON.stringify({}),
	});
	return response.json() as Promise<RemoveSessionResponse>;
}

type ValidateSessionInput = {
	value: string;
};
type ValidateSessionResponse = {
	user: any;
	session: any;
	error?: string;
};

export async function validateClerkSession(input: ValidateSessionInput) {
	const response = await fetch("/api/session/validate", {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
		},
		body: JSON.stringify({ value: input.value }),
	});
	return response.json() as Promise<ValidateSessionResponse>;
}

export async function getClerkSession(clerk: Clerk | null) {
	const sessions = await clerk?.user?.getSessions();
	if (sessions?.length) {
		const value = sessions?.[0].id;
		return { value, sessions };
	}
	return { value: null, sessions: [] };
}
