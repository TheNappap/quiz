
import { onError } from "./utils.js"
import { onEventError,onEventMessage,fetchLatestEvent } from "./event.js"

function onLogin(username) {
	var evtSource = new EventSource('/sse');
	evtSource.onmessage = onEventMessage;
	evtSource.onerror = onEventError;
	window.onbeforeunload = function(){
	   evtSource.onerror = function(){}
	}
	window.localStorage.setItem('Quiz_username', username);
	document.getElementById("login_frame").style.display = "none";
	document.getElementById("user").innerHTML = "Username: "+username;
	fetchLatestEvent();
}

function onRelogin(username) {
	onLogin(username);
}

function onLoginFailed(responseText) {
	onError(responseText)
}

function onReloginFailed() {
	document.getElementById("sub_title").innerHTML = "Choose a username";
	document.getElementById("login_frame").style.display = "table";
}

export function login(){
	var xmlHttp = new XMLHttpRequest();
    xmlHttp.onreadystatechange = function() {
        if (xmlHttp.readyState == 4) {
			if (xmlHttp.status == 202) {
				onLogin(xmlHttp.responseText);
			} else {
				onLoginFailed(xmlHttp.responseText);
			}
		}
    }
    xmlHttp.open("POST", "/login");
    xmlHttp.send(document.getElementById("name").value);
}

window.login = login;

export function relogin(){
	var username = window.localStorage.getItem('Quiz_username');
	if (username == null) {
		onReloginFailed();
		return
	}
	
	var xmlHttp = new XMLHttpRequest();
    xmlHttp.onreadystatechange = function() {
        if (xmlHttp.readyState == 4) {
			if (xmlHttp.status == 202) {
				onRelogin(xmlHttp.responseText);
			} else {
				onReloginFailed();
			}
		}
    }
    xmlHttp.open("POST", "/relogin");
    xmlHttp.send(username);
}