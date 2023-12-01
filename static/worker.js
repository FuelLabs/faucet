let work = false;

onmessage = async function(ev) {
	// Sanitize input
	if(!ev || !ev.data) return;
	
	// If already working, stop
	if(work) {
		work = false;
		return;
	}
	
	const {fuelAddress, difficultyLevel} = ev.data;
	const difficultyMatch = new Array(difficultyLevel).fill('0').join('');
	work = true;

	let i = 0;
	let salt = getRandomSalt();

	console.log("Working", difficultyLevel, fuelAddress);
	
	while(work) {
		let buffer = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(`${fuelAddress}${salt}${i}`));
		let hexString = Array.from(new Uint8Array(buffer)).map(b => b.toString(16).padStart(2, '0')).join('');

		if(hexString.substring(0, difficultyLevel) === difficultyMatch) {
			this.postMessage(`Valid hash: ${hexString}`);
		}
		i++;
	}

	this.postMessage("Finished");
}

function getRandomSalt() {
	// Generate a random salt
    let saltArray = new Uint8Array(32);
    crypto.getRandomValues(saltArray);
    return Array.from(saltArray).map(b => b.toString(16).padStart(2, '0')).join('');
}