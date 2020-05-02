import os

from flask import Flask, request, abort, jsonify, send_from_directory
from flask_cors import CORS

UPLOAD_DIRECTORY = os.path.abspath("cooked_assets/")

if not os.path.exists(UPLOAD_DIRECTORY):
    os.makedirs(UPLOAD_DIRECTORY)


app = Flask(__name__)
CORS(app)

@app.route("/assets/<path:path>")
def get_file(path):
    """Download a file."""

    path = os.path.abspath(os.path.join(UPLOAD_DIRECTORY, path))
    if os.path.commonprefix([path, UPLOAD_DIRECTORY]) != UPLOAD_DIRECTORY:
        abort(400, "outside of sandbox")
    (directory,filename) = os.path.split(os.path.abspath(path))

    return send_from_directory(directory, filename, as_attachment=True)

@app.route("/assets/<filename>", methods=["POST"])
def post_file(filename):
    """Upload a file."""

    path = os.path.abspath(os.path.join(UPLOAD_DIRECTORY, path))
    if os.path.commonprefix([path, UPLOAD_DIRECTORY]) != UPLOAD_DIRECTORY:
        abort(400, "outside of sandbox")
    (directory,filename) = os.path.split(os.path.abspath(path))
    os.makedirs(directory)

    with open(os.path.join(directory, filename), "wb") as fp:
        fp.write(request.data)

    # Return 201 CREATED
    return "", 201


if __name__ == "__main__":
    app.run(debug=True, host="assets.shine.com", port=9100)