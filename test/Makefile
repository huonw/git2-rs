run: git2_test
	RUST_THREADS=1 ./git2_test

git2_test: git2_test.rs
rustc --test git2_test.rs -L .. --link-args="${LINK_ARGS}"

clean:
	rm -rf git2_test *.dylib *.dSYM *.so *.o
