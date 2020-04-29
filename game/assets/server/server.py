import os

from flask import Flask, request, abort, jsonify, send_from_directory

UPLOAD_DIRECTORY = os.path.abspath("cooked_assets/")

if not os.path.exists(UPLOAD_DIRECTORY):
    os.makedirs(UPLOAD_DIRECTORY)


api = Flask(__name__)

@api.route("/assets/<path:path>")
def get_file(path):
    """Download a file."""

    path = os.path.abspath(os.path.join(UPLOAD_DIRECTORY, path))
    if os.path.commonprefix([path, UPLOAD_DIRECTORY]) != UPLOAD_DIRECTORY:
        abort(400, "outside of sandbox")
    (directory,filename) = os.path.split(os.path.abspath(path))

    return send_from_directory(directory, filename, as_attachment=True)

@api.route("/assets/<filename>", methods=["POST"])
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
    api.run(debug=True, port=9100)