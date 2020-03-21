tr = {
    title: "Create Account",

    errors: {
        user: {
            missing: "Please provide a valid username",
            too_short: "Too short, required min length: " + cfg.user.min_len,
            too_long: "Too long, required max length: " + cfg.user.max_len,
            invalid_character: "User name may not contian '@'",
            already_taken: "User name already in use"
        },

        email: {
            invalid: "Invalid email format",
            invalid_domain: "Unsupported email server",
            already_taken: "Email already in use",
        },

        password: {
            missing: "Provide password",
            too_short: "Too short, required min length: " + cfg.password.min_len,
            too_long: "Too long, required max length: " + cfg.password.max_len,
            too_week: "Password is too week"
        },

        terms: {
            missing: "Accept terms"
        },
    }
}