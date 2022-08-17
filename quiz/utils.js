
export function loadTitle(){
	var xmlHttp = new XMLHttpRequest();
    xmlHttp.onreadystatechange = function() { 
        if (xmlHttp.readyState == 4 && xmlHttp.status == 200)
			//document.getElementById("title").innerHTML = JSON.parse(xmlHttp.responseText).title;
			document.getElementById("title").innerHTML = xmlHttp.responseText;
    }
    xmlHttp.open("GET", "/title");
    xmlHttp.send();
}

export function onError(responseText) {
	document.getElementById("error").innerHTML = responseText;
}

export function onInfo(responseText) {
	document.getElementById("info").innerHTML = responseText;
}