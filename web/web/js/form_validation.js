function form_input(elementId) {
    return document.querySelector('#' + elementId + ' input');
}

function form_value(elementId) {
    return form_input(elementId).value
}

function form_set_valid(elementId, silent) {
    console.log('valid: ' + silent + ' ' + elementId);
    var element = form_input(elementId);
    element.classList.remove('is-invalid');
    if (silent) {
        element.classList.remove('is-valid');
    }
    else {
        element.classList.add('is-valid');
    }
    element.setCustomValidity('');

    var element_error = document.querySelector('#' + elementId + ' .invalid-feedback');
    if (element_error) {
        element_error.innerHTML = '';
    }
}

function form_set_invalid(elementId, silent, error_msg) {
    console.log('invalid: ' + silent + ' ' + elementId);
    var element = form_input(elementId);
    element.setCustomValidity(error_msg);
    element.classList.remove('is-valid');
    if (silent) {
        element.classList.remove('is-invalid');
    }
    else {
        element.classList.add('is-invalid');
    }

    var element_error = document.querySelector('#' + elementId + ' .invalid-feedback');
    if (element_error) {
        element_error.innerHTML = error_msg;
    }
}
