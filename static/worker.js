let working = false;

onmessage = async function (ev) {
  // If already working, stop
  if (working) {
    working = false;
    return;
  }

  // Sanitize input
  if (!ev || !ev.data) return;

  const { difficultyLevel, salt } = ev.data;
  const difficultyMatch = new Array(difficultyLevel).fill("0").join("");
  working = true;

  let i = 0;

  console.log("Working", difficultyLevel, salt);

  while (working) {
    let buffer = await crypto.subtle.digest(
      "SHA-256",
      new TextEncoder().encode(`${salt}${i}`)
    );
    let hexString = Array.from(new Uint8Array(buffer))
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");

    if (hexString.substring(0, difficultyLevel) === difficultyMatch) {
      this.postMessage({
        type: "hash",
        value: { salt, nonce: i, hash: hexString },
      });
    }
    i++;
  }

  this.postMessage({ type: "finish" });
};

function getRandomSalt() {
  // Generate a random salt
  let saltArray = new Uint8Array(32);
  crypto.getRandomValues(saltArray);
  return Array.from(saltArray)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}
