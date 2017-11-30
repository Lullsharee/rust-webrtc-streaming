window.onload = setup;
const videoScreen = document.getElementById('camera')
let localStream = null;
let peerConnection = null;
let client_id = 0;

const wsUrl = 'ws://localhost:3333/host';
const ws = new WebSocket(wsUrl);

ws.onopen = function(evt) {
    console.log('ws open()');
};
ws.onerror = function(err) {
    console.error('ws onerror() ERR:', err);
};
ws.onmessage = function(evt) {
    console.log('ws onmessage() data:', evt.data);

    const message = JSON.parse(evt.data);
    //const detail_message = JSON.parse(message.data);
    console.log(message.type)
    if (message.type === 'offer') {
        // offer 受信時
        console.log('Received offer ...');
        //textToReceiveSdp.value = message.data.sdp;
        const offer = new RTCSessionDescription(message);
        client_id = message.client_id
        setOffer(offer);
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
    const message = JSON.stringify({ typed: 'candidate', ice: candidate,id:client_id});
    console.log('sending candidate=' + message);
    ws.send(message);
}

// ICE candidate生成時に送信する
function sendIceCandidate(candidate) {
    console.log('---sending ICE candidate ---');
    const message = JSON.stringify({ typed: 'candidate', ice: candidate,id:client_id});
    console.log('sending candidate=' + message);
    ws.send(message);
}

function startVideo() {
    navigator.mediaDevices.getUserMedia({video: true, audio: true})
        .then(function (stream) { // success
            playVideo(videoScreen,stream);
            localStream = stream;
        }).catch(function (error) { // error
            console.error('mediaDevice.getUserMedia() error:', error);
            return;
    });
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
    if (localStream) {
        console.log('Adding local stream...');
        peer.addStream(localStream);
    }
    else {
        console.warn('no local stream, but continue.');
    }
    return peer;
}

function sendSdp(sessionDescription) {
    console.log('---sending sdp ---');

    const message = JSON.stringify(sessionDescription);
    console.log('sending SDP=' + message);
    const true_text = JSON.stringify({typed:sessionDescription.type,sdp:sessionDescription.sdp,client_id:client_id});
    ws.send(true_text);
}


//function Connect skio

function begin(){
    var msg = {
        name: document.controls.name.value
    }
    ws.send(JSON.stringify(msg));
    changeStatus(true);
    document.controls.start.style.display = "none";
    document.controls.name.disabled = true;
}

function makeAnswer() {
    console.log('sending Answer. Creating remote session description...' );
    if (! peerConnection) {
        console.error('peerConnection NOT exist!');
        return;
    }
    peerConnection.createAnswer()
        .then(function (sessionDescription) {
            console.log('createAnswer() succsess in promise');
            return peerConnection.setLocalDescription(sessionDescription);
        }).then(function() {
            console.log('setLocalDescription() succsess in promise');
            sendSdp(peerConnection.localDescription);
    }).catch(function(err) {
        console.error(err);
    });
}

function onSdpText() {
    const text = textToReceiveSdp.value;
        // Offerを受けた側が相手からのOfferをセットする場合
        console.log('Received offer text...');
        const offer = new RTCSessionDescription({
            typed : 'offer',
            sdp : text,
        });
        setOffer(offer);
    textToReceiveSdp.value ='';
}

function setOffer(sessionDescription) {
    if (peerConnection) {
        console.error('peerConnection alreay exist!');
    }
    peerConnection = prepareNewConnection();
    peerConnection.setRemoteDescription(sessionDescription)
    makeAnswer();
    // peerConnection.onnegotiationneeded = function () {
    //     peerConnection.setRemoteDescription(sessionDescription)
    //         .then(function() {
    //             console.log('setRemoteDescription(offer) succsess in promise');
    //             makeAnswer();
    //         }).catch(function(err) {
    //             console.error('setRemoteDescription(offer) ERROR: ', err);
    //     });
//}
}

function setup(){
    startVideo();
    document.controls.start.addEventListener("click", begin);
    document.controls.name.disabled = false;
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

function hangUp(){
    if (peerConnection) {
        if(peerConnection.iceConnectionState !== 'closed'){
            peerConnection.close();
            peerConnection = null;
            const message = JSON.stringify({ typed: 'close' });
            console.log('sending close message');
            ws.send(message);
            cleanupVideoElement(remoteVideo);
            textForSendSdp.value = '';
            textToReceiveSdp.value = '';
            return;
        }
    }
    console.log('peerConnection is closed.');
}
function cleanupVideoElement(element) {
    element.pause();
    element.srcObject = null;
}
