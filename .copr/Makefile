SRPM_DIR := $(shell rpm --eval '%{_srcrpmdir}')

srpm:
	rpmbuild -bs dist/rpm/fedora.spec
	mv $(SRPM_DIR)/*.src.rpm $(outdir)
