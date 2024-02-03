var sender_id = "";
var userMap = {};
var ws_id = "";
var username = "";
var socket: WebSocket;

enum MessageTypes {
    SetUsername,
    AddRoom,
    ChangeRoom,
    RemoveFromRoom,
    Basic,
};

type BasicMessage = { /* ... */ };
type ImageMessage = { /* ... */ };
type NotificationMessage = { /* ... */ };
type TypingMessage = { /* ... */ };
type UserRemovalMessage = { /* ... */ };
type ChangeRoomMessage = { /* ... */ };
type UsernameChangeMessage = { /* ... */ };
type CreateRoomChangeMessage = { /* ... */ };

type Message = 
    | { type: 'Basic', message: BasicMessage }
    | { type: 'Image', message: ImageMessage }
    | { type: 'Notification', message: NotificationMessage }
    | { type: 'Typing', message: TypingMessage }
    | { type: 'UserRemoval', message: UserRemovalMessage }
    | { type: 'ChangeRoom', message: ChangeRoomMessage }
    | { type: 'UsernameChange', message: UsernameChangeMessage }
    | { type: 'CreateRoomChange', message: CreateRoomChangeMessage };

/*
    fetch('/get-ip')
    .then(response => response.json())
    .then(data => {
        const serverIp = data.ip;
        console.log('Server IP:', serverIp);
        // Initialize WebSocket connection after retrieving the server IP
        initializeWebSocket(serverIp);
    })
    .catch(error => console.error('Error fetching server IP:', error));

function initializeWebSocket(serverIp: string): void {
    socket = new WebSocket(`ws://${serverIp}:8080/ws/`);

    socket.onclose = function(event) {
        if (!event.wasClean) {
        window.location.href = '/login'; // Redirect to login page
        }
    };

    socket.onmessage = function(event) {
        console.log(event.data);
        const message = JSON.parse(event.data);

        // Handle user list message
        if (message.message_type === "user_list") {
            userMap = message.user_hashmap;
        }

        // Handle initial setup message
        if (message.type === "init") {
            sender_id = message.user_id;
            ws_id = message.ws_id;
            username = message.username;
            return;
        }

        if (message.type === "new_user_joined") {
            // adds new user UUID and username received from the server
            userMap[message.user_id] = message.username;
            return;
        }

        if (message.type === "update_username") {
            const message_sender_id = message.sender_id;
            const new_username = message.new_username;
            const old_username = userMap[message_sender_id];

            retroactivelyChangeUsername(old_username, new_username);
            userMap[message.sender_id] = message.new_username;
            console.log("HEY");
            console.log("message_sender: ", message_sender_id);
            console.log("Sender_id: ", sender_id);
            if (message_sender_id === sender_id) {
                username = new_username;
                console.log("YAY");
            }
        }

        // Check if there's a 'logout' command from the server
        if (message.type === "logout") {
            // Redirect to the logout endpoint
            window.location.href = '/logout';
        }

        if (message.message_type === "Basic") {
            if (message.ws_id === ws_id) {
                return;
            }

            const chatContainer = document.getElementById('chat-container');
            const messageWrapper = document.createElement('div');
            const usernameElement = document.createElement('div');
            const messageElement = document.createElement('div');

            usernameElement.textContent = (userMap[message.sender_id]) + ':';
            usernameElement.classList.add('username');
            messageElement.textContent = message.content;

            messageWrapper.appendChild(usernameElement);
            messageWrapper.appendChild(messageElement);
            messageWrapper.classList.add('chat-message');

            if (message.sender_id === sender_id) {
                messageWrapper.classList.add('sent-message');
            } else {
                messageWrapper.classList.add('received-message');
            }
            if (chatContainer){
                chatContainer.appendChild(messageWrapper);
                chatContainer.scrollTop = chatContainer.scrollHeight;
            }
        }
    };


    function retroactivelyChangeUsername(oldUsername: string, newUsername: string) {
        const chatContainer = document.getElementById('chat-container');
        if (chatContainer) {
            const messages = chatContainer.getElementsByClassName('chat-message');
            for (let message of messages) {
                const usernameElement = message.querySelector('.username');
                if (usernameElement && usernameElement.textContent?.startsWith(oldUsername + ':')) {
                    usernameElement.textContent = newUsername + ':';
                }
            }
        }
    }


    document.getElementById('Message')!.addEventListener('submit', function(event: Event) {
        event.preventDefault();
        sendMessage();
    });
    
    function sendMessage(): void {
        const textarea = document.querySelector('textarea[name="message_form"]') as HTMLTextAreaElement;
        const messageContent = textarea.value.trim();
        
        if (messageContent !== '') {
        const payload = {
            content: messageContent,
            sender_id: sender_id,
            message_type: MessageTypes.Basic
        };
        
        if (socket.readyState === WebSocket.OPEN) {
            socket.send(JSON.stringify(payload));
        }
        }
        
        const chatContainer = document.getElementById('chat-container') as HTMLElement;
        const messageWrapper = document.createElement('div');
        const usernameElement = document.createElement('div');
        const messageElement = document.createElement('div');
        
        usernameElement.textContent = (username) + ':';
        usernameElement.classList.add('username');
        
        messageElement.textContent = textarea.value.trim();
        
        messageWrapper.appendChild(usernameElement);
        messageWrapper.appendChild(messageElement);
        messageWrapper.classList.add('chat-message');
        messageWrapper.classList.add('sent-message');
        
        chatContainer.appendChild(messageWrapper);
        chatContainer.scrollTop = chatContainer.scrollHeight;
        
        textarea.value = '';
    }
    

    let textArea = document.querySelector('textarea[name="message_form"]') as HTMLTextAreaElement;
    textArea.addEventListener('keydown', function(event: KeyboardEvent) {
        if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            sendMessage();
        }
    });
    

    document.getElementById('Username')!.addEventListener('submit', function(event) {
        event.preventDefault();
        const usernameField = document.getElementById('usernameField') as HTMLInputElement;

        if (usernameField) {
            const newUsername = usernameField.value.trim();

            if (newUsername !== '') {
                const payload = {
                    sender_id: sender_id,
                    content: newUsername,
                    message_type: MessageTypes.SetUsername
                };

                if (socket.readyState === WebSocket.OPEN) {
                    socket.send(JSON.stringify(payload));
                }
            }
        }
    });

    document.getElementById('Username')!.addEventListener('submit', function(event) {
        event.preventDefault();
        const usernameField = document.getElementById('usernameField') as HTMLInputElement;

        if (usernameField) {
            const newUsername = usernameField.value.trim();

            if (newUsername !== '') {
                const payload = {
                    sender_id: sender_id,
                    content: newUsername,
                    message_type: MessageTypes.SetUsername
                };

                if (socket.readyState === WebSocket.OPEN) {
                    socket.send(JSON.stringify(payload));
                }
            }
        }
    });
}

*/
fetch('/get-ip')
    .then(response => response.json())
    .then(data => {
        const serverIp = data.ip;
        console.log('Server IP:', serverIp);
        // Initialize WebSocket connection after retrieving the server IP
        initializeWebSocket(serverIp);
    })
    .catch(error => console.error('Error fetching server IP:', error));

function initializeWebSocket(serverIp: string): void {
    socket = new WebSocket(`ws://${serverIp}:8080/ws/`);

    socket.onclose = function(event) {
        if (!event.wasClean) {
        window.location.href = '/login'; // Redirect to login page
        }
    };

    socket.onmessage = function(event) {
        console.log(event.data);
        const message = JSON.parse(event.data);

        // Handle user list message
        if (message.message_type === "user_list") {
            userMap = message.user_hashmap;
        }

        // Handle initial setup message
        if (message.type === "init") {
            sender_id = message.user_id;
            ws_id = message.ws_id;
            username = message.username;
            return;
        }

        if (message.type === "new_user_joined") {
            // adds new user UUID and username received from the server
            userMap[message.user_id] = message.username;
            return;
        }

        if (message.type === "update_username") {
            const message_sender_id = message.sender_id;
            const new_username = message.new_username;
            const old_username = userMap[message_sender_id];

            retroactivelyChangeUsername(old_username, new_username);
            userMap[message.sender_id] = message.new_username;
            console.log("HEY");
            console.log("message_sender: ", message_sender_id);
            console.log("Sender_id: ", sender_id);
            if (message_sender_id === sender_id) {
                username = new_username;
                console.log("YAY");
            }
        }

        // Check if there's a 'logout' command from the server
        if (message.type === "logout") {
            // Redirect to the logout endpoint
            window.location.href = '/logout';
        }

        if (message.message_type === "Basic") {
            if (message.ws_id === ws_id) {
                return;
            }

            const chatContainer = document.getElementById('chat-container');
            const messageWrapper = document.createElement('div');
            const usernameElement = document.createElement('div');
            const messageElement = document.createElement('div');

            usernameElement.textContent = (userMap[message.sender_id]) + ':';
            usernameElement.classList.add('username');
            messageElement.textContent = message.content;

            messageWrapper.appendChild(usernameElement);
            messageWrapper.appendChild(messageElement);
            messageWrapper.classList.add('chat-message');

            if (message.sender_id === sender_id) {
                messageWrapper.classList.add('sent-message');
            } else {
                messageWrapper.classList.add('received-message');
            }
            if (chatContainer){
                chatContainer.appendChild(messageWrapper);
                chatContainer.scrollTop = chatContainer.scrollHeight;
            }
        }
    };


    function retroactivelyChangeUsername(oldUsername: string, newUsername: string) {
        const chatContainer = document.getElementById('chat-container');
        if (chatContainer) {
            const messages = chatContainer.getElementsByClassName('chat-message');
            for (let message of messages) {
                const usernameElement = message.querySelector('.username');
                if (usernameElement && usernameElement.textContent?.startsWith(oldUsername + ':')) {
                    usernameElement.textContent = newUsername + ':';
                }
            }
        }
    }


    document.getElementById('Message')!.addEventListener('submit', function(event: Event) {
        event.preventDefault();
        sendMessage();
    });
    
    function sendMessage(): void {
        const textarea = document.querySelector('textarea[name="message_form"]') as HTMLTextAreaElement;
        const messageContent = textarea.value.trim();
        
        if (messageContent !== '') {
        const payload = {
            content: messageContent,
            sender_id: sender_id,
            message_type: MessageTypes.Basic
        };
        
        if (socket.readyState === WebSocket.OPEN) {
            socket.send(JSON.stringify(payload));
        }
        }
        
        const chatContainer = document.getElementById('chat-container') as HTMLElement;
        const messageWrapper = document.createElement('div');
        const usernameElement = document.createElement('div');
        const messageElement = document.createElement('div');
        
        usernameElement.textContent = (username) + ':';
        usernameElement.classList.add('username');
        
        messageElement.textContent = textarea.value.trim();
        
        messageWrapper.appendChild(usernameElement);
        messageWrapper.appendChild(messageElement);
        messageWrapper.classList.add('chat-message');
        messageWrapper.classList.add('sent-message');
        
        chatContainer.appendChild(messageWrapper);
        chatContainer.scrollTop = chatContainer.scrollHeight;
        
        textarea.value = '';
    }
    

    let textArea = document.querySelector('textarea[name="message_form"]') as HTMLTextAreaElement;
    textArea.addEventListener('keydown', function(event: KeyboardEvent) {
        if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            sendMessage();
        }
    });
    

    document.getElementById('Username')!.addEventListener('submit', function(event) {
        event.preventDefault();
        const usernameField = document.getElementById('usernameField') as HTMLInputElement;

        if (usernameField) {
            const newUsername = usernameField.value.trim();

            if (newUsername !== '') {
                const payload = {
                    sender_id: sender_id,
                    content: newUsername,
                    message_type: MessageTypes.SetUsername
                };

                if (socket.readyState === WebSocket.OPEN) {
                    socket.send(JSON.stringify(payload));
                }
            }
        }
    });

    document.getElementById('Username')!.addEventListener('submit', function(event) {
        event.preventDefault();
        const usernameField = document.getElementById('usernameField') as HTMLInputElement;

        if (usernameField) {
            const newUsername = usernameField.value.trim();

            if (newUsername !== '') {
                const payload = {
                    sender_id: sender_id,
                    content: newUsername,
                    message_type: MessageTypes.SetUsername
                };

                if (socket.readyState === WebSocket.OPEN) {
                    socket.send(JSON.stringify(payload));
                }
            }
        }
    });
}