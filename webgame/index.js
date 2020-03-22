const rust = import('./pkg/shine_webgame');
const canvas = document.getElementById('gameCanvas');
const gl = canvas.getContext("webgl", { antialias: true });

rust.then(m => {
    if (!gl) {
        alert('Failed to initialize WebGL');
        return;
    }

    const game = new m.WebGame();
    console.log(game);
    /*const FPS_THROTTLE = 1000.0 / 30.0; // milliseconds / frames
    const initialTime = Date.now();
    let lastDrawTime = -1;// In milliseconds*/

    function render() {
        window.requestAnimationFrame(render);
        /*const currTime = Date.now();

        if (currTime >= lastDrawTime + FPS_THROTTLE) {
            lastDrawTime = currTime;

            if (window.innerHeight !== canvas.height || window.innerWidth !== canvas.width) {
                canvas.height = window.innerHeight;
                canvas.clientHeight = window.innerHeight;
                canvas.style.height = window.innerHeight;

                canvas.width = window.innerWidth;
                canvas.clientWidth = window.innerWidth;
                canvas.style.width = window.innerWidth;

                gl.viewport(0, 0, window.innerWidth, window.innerHeight);
            }

            let elapsedTime = currTime - initialTime;
            dougsClient.update(elapsedTime, window.innerHeight, window.innerWidth);*/
        game.render();
    }

    render();
});
