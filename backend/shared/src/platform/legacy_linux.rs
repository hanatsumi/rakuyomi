#[macro_export]
macro_rules! syscall {
    ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
        #[allow(unused_unsafe)]
        let res = unsafe { libc::$fn($($arg, )*) };
        if res < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }
    }};
}

#[macro_export]
macro_rules! generate_syscall_shims {
    () => {
        #[no_mangle]
        pub unsafe extern "C" fn epoll_create1(flag: libc::c_int) -> libc::c_int {
            use libc;

            let ep = $crate::syscall!(syscall(libc::SYS_epoll_create1, flag))
                .map(|fd| fd as libc::c_int)
                .or_else(|e| {
                    match e.raw_os_error() {
                        Some(libc::ENOSYS) => {
                            // Using epoll_create() followed by fcntl() instead of epoll_create1() with EPOLL_CLOEXEC
                            // flag for backwards compatibility.
                            let ep = $crate::syscall!(syscall(libc::SYS_epoll_create, 1024))?
                                as libc::c_int;
                            $crate::syscall!(fcntl(ep, libc::F_SETFD, libc::FD_CLOEXEC))?;

                            Ok(ep)
                        }
                        _ => Err(e),
                    }
                });

            match ep {
                Ok(fd) => fd,
                Err(e) => e.raw_os_error().expect("expected to have raw os error"),
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn eventfd2(
            initval: libc::c_uint,
            flags: libc::c_int,
        ) -> libc::c_int {
            use libc;

            let fd = $crate::syscall!(syscall(libc::SYS_eventfd2, initval, flags))
                .map(|fd| fd as libc::c_int)
                .or_else(|e| {
                    match e.raw_os_error() {
                        Some(libc::ENOSYS) => {
                            // Fall back to eventfd() for older systems
                            let fd = $crate::syscall!(syscall(libc::SYS_eventfd, initval))
                                .map(|fd| fd as libc::c_int)?;

                            // Apply CLOEXEC if requested
                            if flags & libc::EFD_CLOEXEC != 0 {
                                $crate::syscall!(fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC))?;
                            }

                            // Apply NONBLOCK if requested
                            if flags & libc::EFD_NONBLOCK != 0 {
                                $crate::syscall!(fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK))?;
                            }

                            Ok(fd)
                        }
                        _ => Err(e),
                    }
                });

            match fd {
                Ok(fd) => fd,
                Err(e) => e.raw_os_error().expect("expected to have raw os error"),
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn eventfd(initval: libc::c_uint) -> libc::c_int {
            use libc;

            let fd =
                $crate::syscall!(syscall(libc::SYS_eventfd, initval)).map(|fd| fd as libc::c_int);

            match fd {
                Ok(fd) => fd,
                Err(e) => e.raw_os_error().expect("expected to have raw os error"),
            }
        }
    };
}
