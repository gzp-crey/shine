const rust = import('./pkg/shine_webgame');
const canvas = document.getElementById('gameCanvas');
const gl = canvas.getContext("webgl", { antialias: true });



rust
    .then(
        async m => {
            if (!gl) {
                alert('Failed to initialize WebGL');
                return;
            }

            const game = new m.WebGame;
            console.log(game);
            const gameView = await game.create_render('gameCanvas');
            console.log(render);
        })
    .catch(
        error => {
            alert('Failed to initialize game', error);
        });
