// export const socket = new WebSocket("ws://localhost:1234
export function connect() {
    const socket = new WebSocket("ws://localhost:1234");

    console.log("MyTest");

    socket.addEventListener("open", (event) => {
        console.log("Connected to WebSocket server");
        socket.send("Hello from client");
    });

    socket.addEventListener("close", (event) => {
        console.log("Disconnected from WebSocket server");
    });

    socket.addEventListener("error", (event) => {
        console.error("WebSocket error:", event);
    });

    return socket;
}


export function waitForMessage(socket) {
    return new Promise((resolve) => {
        socket.onmessage = (event) => {
            resolve(event.data);
        };
    });
}