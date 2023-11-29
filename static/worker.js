var num;
var isPrime = false;
var firstMessagePostCompleted = false;
onmessage = function(ev) {
	num = parseInt(ev.data);
	let temp = num;
	if((temp <= 1) || (isNaN(temp))){
		this.setTimeout(() => {
			//this.postMessage("<b>" + temp + "</b>");
			this.postMessage("<b>Primes are integers greater than one with no positive divisors besides one and itself. Please enter a number >= 2.</br>Reload to continue...</b>");
		}, 300);
		firstMessagePostCompleted = true;
	}
	else if(temp == 2){
		isPrime = true;
		this.setTimeout(() => {
			this.postMessage(temp);
		}, 500);
		firstMessagePostCompleted = true;
	}
	else if(!firstMessagePostCompleted && !isPrime){
		let ctr = 0;
		temp += 1;
		for(let i = 2; i<=Math.sqrt(temp); i++){
			if(temp % i == 0){
				ctr++;
				break;
			}
		}
		if(ctr > 0){
			this.setTimeout(() => {
				//this.postMessage("<b>" + temp + "</b>");
				this.postMessage(temp);
			}, 500);
			firstMessagePostCompleted = true;		
		}
		else{
			isPrime = true;
		}
	}
	else if(firstMessagePostCompleted && !isPrime){
		let ctr = 0;
		for(let i = 2; i<=Math.sqrt(temp); i++){
			if(temp % i == 0){
				ctr++;
				break;
			}
		}
		if(ctr > 0){
			this.setTimeout(() => {
				//this.postMessage("<b>" + temp + "</b>");
				this.postMessage(temp);
			}, 500);
		}
		else{
			isPrime = true;
		}
	}
	else if(firstMessagePostCompleted && isPrime){
		this.setTimeout(() => {
			//this.postMessage("<b>" + temp + "</b>");
			this.postMessage("The next prime number after your input is : " + "<b>" + temp + "</b></br>End of program.</br>------------------------------------------------------------------------------------------------------------------------------");
		}, 300);	
	}	
}