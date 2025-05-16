
import { onError,onInfo } from "./utils.js"

export function onEventError(ev) {
	console.log("SSE Error:");
	console.log(ev);
}

export function onEventMessage(ev) {
	onError("");
	onInfo("");
	console.log("Received event: " + JSON.stringify(ev));

	document.getElementById("image").innerHTML = "";
	var data = JSON.parse(ev.data);
	if (data.Lobby) {
		onLobby(data.Lobby);
	} else if (data.Question) {
		onQuestion(data.Question);
	} else if (data.Ranking) {
		onRanking(data.Ranking);
	} else if (data == "Finished") {
		onFinished();
	} else {
		console.log("Unknown event: " + ev.data);
	}
}

export function fetchLatestEvent() {
	var username = window.localStorage.getItem('Quiz_username');
	if (username == null) {
		return
	}
	var xmlHttp = new XMLHttpRequest();
    xmlHttp.onreadystatechange = function() {
        if (xmlHttp.readyState == 4) {
			if (xmlHttp.status == 200) {
				var ev = { data: xmlHttp.responseText};
				onEventMessage(ev);
			} else {
				onError(xmlHttp.responseText);
			}
		}
    }
    xmlHttp.open("POST", "/last_event");
    xmlHttp.send(username);
}

function onLobby(lobby) {
	document.getElementById("sub_title").innerHTML = "Waiting for quiz to start...";
	document.getElementById("main_frame").innerHTML = "";
	var table = document.createElement('table');
	var tr = document.createElement('tr');
	tr.innerHTML = "<tr><th>Users</th></tr>";
	table.appendChild(tr);
	for (var user of lobby.users) {
		var tr = document.createElement('tr');
		var td = document.createElement('td');
		td.innerHTML = user;
		tr.appendChild(td);
		table.appendChild(tr);
	}
	document.getElementById("main_frame").appendChild(table);
}

function onQuestion(question) {
	document.getElementById("main_frame").innerHTML = "";
	document.getElementById("sub_title").innerHTML = question.title;
	document.getElementById("q_nr").innerHTML = (question.id+1) + "/" + question.total;
	if (question.image) {
		var img = document.createElement("img");
		img.src = question.image;
		document.getElementById("image").appendChild(img);
	}
	if (question.question_type.MultiChoice) {
		onMultiChoice(question.title, question.question_type.MultiChoice);
	} else if (question.question_type.MultiOption) {
		onMultiOption(question.title, question.question_type.MultiOption);
	} else if (question.question_type == "Open") {
		onOpen(question.title);
	} else {
		console.log("Unknown question: ");
		console.log(question);
		document.getElementById("sub_title").innerHTML = "Received unknown question type.";
		return;
	}
}

function onMultiChoice(title, multichoice) {
	for (const [index,option] of multichoice.entries()) {
		var input = document.createElement("input");
		input.type = "radio";
		input.id = option;
		input.name = "question";
		input.value = index;
		document.getElementById("main_frame").appendChild(input);
		document.getElementById("main_frame").innerHTML += " " + option + "</br>" + "</br>";
	}
	var submit = document.createElement("button");
	submit.innerHTML = "Submit";
	submit.onclick = function() {
		for (var input of document.getElementsByName("question")) {
			if (input.checked) {
				submitAnswer(title, { "MultiChoice": Number(input.value) });
				return;
			}
		}
	}
	document.getElementById("main_frame").appendChild(submit);
}

function onMultiOption(title, multioption) {
	for (const [index,option] of multioption.entries()) {
		var input = document.createElement("input");
		input.type = "checkbox";
		input.id = option;
		input.name = "question";
		input.value = index;
		document.getElementById("main_frame").appendChild(input);
		document.getElementById("main_frame").innerHTML += " " + option + "</br>" + "</br>";
	}
	var submit = document.createElement("button");
	submit.innerHTML = "Submit";
	submit.onclick = function() {
		var answers = [];
		for (var input of document.getElementsByName("question")) {
			if (input.checked) {
				answers.push(Number(input.value))
			}
		}
		if (answers.length) {
			submitAnswer(title, { "MultiOption": answers });
		}
	}
	document.getElementById("main_frame").appendChild(submit);
}

function onOpen(title) {
	var textareaObj = document.createElement("textarea");
	textareaObj.id = title;
	textareaObj.placeholder = "Answer";
	document.getElementById("main_frame").appendChild(textareaObj);
	document.getElementById("main_frame").innerHTML += "</br>"
	var submit = document.createElement("button");
	submit.innerHTML = "Submit";
	submit.onclick = function() {
		var textareaObj = document.getElementById(title);
		if (textareaObj.value != "") {
			submitAnswer(title, { "Open": textareaObj.value });
		}
	}
	document.getElementById("main_frame").appendChild(submit);
}

function submitAnswer(title, answer) {
	var username = window.localStorage.getItem('Quiz_username');
	if (username == null) {
		return
	}
	let answerObject = { user: username, question: title, answer: answer };
	
	var xmlHttp = new XMLHttpRequest();
    xmlHttp.onreadystatechange = function() {
        if (xmlHttp.readyState == 4) {
			if (xmlHttp.status == 202) {
				onInfo("Submitted answer: " + xmlHttp.responseText);
			} else {
				onError(xmlHttp.responseText);
			}
		}
    }
	
    xmlHttp.open("POST", "/submit_answer");
    xmlHttp.send(JSON.stringify(answerObject));
}

function onRanking(ranking) {
	document.getElementById("sub_title").innerHTML = "Ranking";
	document.getElementById("q_nr").innerHTML = "";
	document.getElementById("main_frame").innerHTML = "";
	var table = document.createElement('table');
	var tr = document.createElement('tr');
	tr.innerHTML = "<tr><th></th><th>User</th><th>Score</th></tr>";
	table.appendChild(tr);
	for (var [index,score] of ranking.scores.entries()) {
		var tr = document.createElement('tr');
		score.unshift((index+1) + ".");
		score[2] += "/" + ranking.max_score;
		for (var el of score) {
			var td = document.createElement('td');
			td.innerHTML = el;
			tr.appendChild(td);
		}
		table.appendChild(tr);
	}
	document.getElementById("main_frame").appendChild(table);
}

function onFinished() {
	document.getElementById("sub_title").innerHTML = "No more questions, waiting for host to share ranking...";
	document.getElementById("q_nr").innerHTML = "";
	document.getElementById("main_frame").innerHTML = "";
}