import Clerk from "@clerk/clerk-js";

type DispenseMethod = "auth" | "pow";
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

type CreateSessionInput = {
	address?: string | null;
};
type CreateSessionResponse = {
	status: string;
	salt: string;
	difficulty: number;
	error?: string;
};

export async function createSession(payload: CreateSessionInput) {
	if (!payload.address) {
		throw new Error("No address provided");
	}

	const response = await fetch("/api/session", {
		method: "POST",
		headers: {
			Accept: "application/json",
			"Content-Type": "application/json",
		},
		body: JSON.stringify(payload),
	});

	return response.json() as Promise<CreateSessionResponse>;
}

type GetSessionInput = {
	salt: string;
};
type GetSessionResponse = {
	address: string;
	error?: string;
};

export async function getSession(payload: GetSessionInput) {
	const response = await fetch("/api/session", {
		method: "GET",
		headers: {
			Accept: "application/json",
			"Content-Type": "application/json",
		},
		body: JSON.stringify(payload),
	});

	return response.json() as Promise<GetSessionResponse>;
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
