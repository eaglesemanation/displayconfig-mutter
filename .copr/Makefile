SRPM_DIR := $(shell rpm --eval '%{_srcrpmdir}')

srpm:
	rpmbuild -bs fedora/displayconfig-mutter.spec
	mv $(SRPM_DIR)/*.src.rpm $(outdir)
