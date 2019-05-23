# Contributing

To contribute to this project first fork the repository and then create PR against the master branch.

Explain the problem your PR solves and solve only for that problem.
If you find more problems while writing your PR, open another PR.

Your PR will be squashed so please write proper commit messages that document the problem that you fix.

Please avoid sending a pull request with recursive merge nodes, as they are impossible to fix once merged.
Please rebase your branch on master instead of merging it.
```
git remote add upstream git@github.com:jean-airoldie/libzmq-rs.git
git fetch upstream
git rebase upstream/master
git push -f
```

If you extend the public API, make sure to document it as well as test it.
It is recommanded to write your test as a documentation test, this way you both test and document
at the same time.
