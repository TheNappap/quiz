# Quiz app

A small server backend writtin in async Rust for providing questions, answering and grading answers.

Features:
* Ability to load own quiz with multi-choice, multi-option and open questions.
* Theoretically unlimited users (but probably not practical at a certain point)
* Automatic grading for all but open questions.
* Manual grading of questions.
* Locking questions to prevent accepting new answers.
* Redoing a question
* Creating and importing backups
* Including a simple sample frontend.
