
const videoScreen = document.getElementById('camera')
const localVideo = document.getElementById('local_video');
let remoteStream = null;
let peerConnection = null;
let localStream = null;
const id = Number(window.location.search.split("=")[1]);
console.log(id);
const title = window.location.hash.slice(1);
const mediaConstraints = {'mandatory': {'OfferToReceiveAudio':true, 'OfferToReceiveVideo':true }};

const ws = new WebSocket("ws://localhost:3333/view?id=" + id);

ws.onopen = function(evt) {
    console.log('ws open()');
    document.getElementById("title").innerHTML = title;
    document.title = title + " \u2014 Stream";
};
ws.onerror = function(err) {
    console.error('ws onerror() ERR:', err);
};
ws.onmessage = function(evt) {
    console.log('ws onmessage() data:', evt.data);

    const message = JSON.parse(evt.data);
    console.log(message.type)
    if (message.type === 'answer') {
        // answer 受信時
        console.log('Received answer ...');

        const answer = new RTCSessionDescription(message);
        setAnswer(answer);
    }
    else if (message.type === 'candidate') {
        // ICE candidate 受信時
        console.log('Received ICE candidate ...');
        const candidate = new RTCIceCandidate(message.ice);
        console.log(candidate);
        addIceCandidate(candidate);
    }
    else if (message.type === 'close') {
        // closeメッセージ受信時
        console.log('peer is closed ...');
        hangUp();
    }
};

// ICE candaidate受信時にセットする
function addIceCandidate(candidate) {
    if (peerConnection) {
        peerConnection.addIceCandidate(candidate);
    }
    else {
        console.error('PeerConnection not exist!');
        return;
    }
}
// ICE candidate生成時に送信する
function sendIceCandidate(candidate) {
    console.log('---sending ICE candidate ---');
    const message = JSON.stringify({ typed: 'candidate', ice: candidate,id:id});
    console.log('sending candidate=' + message);
    ws.send(message);
}

// ICE candidate生成時に送信する
function sendIceCandidate(candidate) {
    console.log('---sending ICE candidate ---');
    const message = JSON.stringify({ typed: 'candidate', ice: candidate,id:id});
    console.log('sending candidate=' + message);
    ws.send(message);
}



// Videoの再生を開始する
function playVideo(element, stream) {
    element.srcObject = stream;
    element.play();
}

function prepareNewConnection() {
    // RTCPeerConnectionを初期化する
    const pc_config = {"iceServers":[ {"urls":"stun:stun.skyway.io:3478"} ]};
    const peer = new RTCPeerConnection(pc_config);
    // ICE Candidateを収集したときのイベント
    if ('ontrack' in peer) {
        peer.ontrack = function(event) {
            console.log('-- peer.ontrack()');
            playVideo(videoScreen, event.streams[0]);
        };
    }
    else {
        peer.onaddstream = function(event) {
            console.log('-- peer.onaddstream()');
            playVideo(videoScreen, event.stream);
        };
    }
    peer.onicecandidate = function (evt) {
        if (evt.candidate) {
            console.log(evt.candidate);
            sendIceCandidate(evt.candidate);
        } else {
            console.log('empty ice event');
            // sendSdp(peer.localDescription);
        }
    };

    // ICEのステータスが変更になったときの処理
    peer.oniceconnectionstatechange = function() {
        console.log('ICE connection Status has changed to ' + peer.iceConnectionState);
        switch (peer.iceConnectionState) {
            case 'closed':
            case 'failed':
                // ICEのステートが切断状態または異常状態になったら切断処理を実行する
                if (peerConnection) {
                    hangUp();
                }
                break;
            case 'dissconnected':
                break;
        }
    };

    // ローカルのストリームを利用できるように準備する
    return peer;
}

function sendSdp(sessionDescription) {
    console.log('---sending sdp ---');
    const message = JSON.stringify(sessionDescription);
    console.log('sending SDP=' + message);

    const true_text = JSON.stringify({typed:sessionDescription.type,sdp:sessionDescription.sdp,stream_id:id});
    ws.send(true_text);
}


//function Connect skio



function makeOffer() {
    peerConnection = prepareNewConnection();
        peerConnection.createOffer(function (sessionDescription) { 
            // in case of success
            peerConnection.setLocalDescription(sessionDescription);
          sendSdp(peerConnection.localDescription);
          }, function () { // in case of error
            console.log("Create Offer failed");
          }, mediaConstraints);
}


function onSdpText() {

        // Offerを受けた側が相手からのOfferをセットする場合
        console.log('Received offer text...');
        const offer = new RTCSessionDescription({
            typed : 'offer',
            sdp : text,
        });
        setOffer(offer);
}

function setAnswer(sessionDescription) {
    if (! peerConnection) {
        console.error('peerConnection NOT exist!');
        return;
    }
    peerConnection.setRemoteDescription(sessionDescription)
        .then(function() {
            console.log('setRemoteDescription(answer) succsess in promise');
        }).catch(function(err) {
            console.error('setRemoteDescription(answer) ERROR: ', err);
    });
}


function connect(){

    if(! peerConnection){
        console.log('make offer');
       
        makeOffer();
    }
    else {
        console.warn('peer already exist.');
    }
}

function changeStatus(online) {
    var on = document.getElementById("status_online");
    var off = document.getElementById("status_offline");
    
    if(online) {
      on.style.display = "inline";
      off.style.display = "none";
      document.title = document.controls.name.value + " \u2014 Stream Online";
    }
    else {
      on.style.display = "none";
      off.style.display = "inline";
      document.title = "Stream Offline";
    }
  }
  
  function verifyName() {
    if(document.controls.name.value.length > 2) {
      document.controls.start.disabled = false;
    }
    else {
      document.controls.start.disabled = true;
    }
  }

  function sleep(time, callback){
	setTimeout(callback, time);
}