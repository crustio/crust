THE SOFTWARE IS PROVIDED "AS IS" AND BRIAN SMITH AND THE AUTHORS DISCLAIM
ALL WARRANTIES WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES
OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL BRIAN SMITH OR THE AUTHORS
BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY
DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN
AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.



*ring*
======

*ring* is focused on the implementation, testing, and optimization of a core
set of cryptographic operations exposed via an easy-to-use (and hard-to-misuse)
API. *ring* exposes a [Rust](https://www.rust-lang.org/) API and is written in
a hybrid of Rust, C, and assembly language.

Particular attention is being paid to making it easy to build and integrate
*ring* into applications and higher-level frameworks, and to ensuring that
*ring* works optimally on small devices, and eventually microcontrollers, to
support Internet of Things (IoT) applications.

*ring* is focused on general-purpose cryptography. WebPKI X.509 certificate
validation is done in the [webpki](https://github.com/briansmith/webpki)
project, which is built on top of *ring*. Also, multiple groups are working on
implementations of cryptographic protocols like TLS, SSH, and DNSSEC on top of
*ring*.

*ring* is the successor of an earlier project called GFp. GFp implemented some
elliptic curve cryptography over prime finite fields, also known as prime
Galois fields and often denoted GF(p). When we implemented RSA, the name GFp
did not make as much sense, since modular arithmetic over RSA public moduli is
not GF(p) arithmetic but rather finite commutative *ring* arithmetic. Also note
that *ring* started as a subset of BoringSSL, and “*ring*” is a substring of
“Bo*ring*SSL”.

Most of the C and assembly language code in *ring* comes from BoringSSL, and
BoringSSL is derived from OpenSSL. *ring* merges changes from BoringSSL
regularly. Also, several changes that were developed for *ring* have already
been merged into BoringSSL.




Documentation
-------------

See the documentation at
https://briansmith.org/rustdoc/ring/.

See [BUILDING.md](BUILDING.md) for instructions on how to build it. These
instructions are especially important on Windows when not building from
crates.io, as there are build prerequisites that need to be installed.



Benchmarks
----------

*ring*'s benchmarks are in the
[crypto-bench](https://github.com/briansmith/crypto-bench) project. Because
there is lots of platform-specific code in *ring*, and because *ring* chooses
dynamically at runtime which optimized implementation of each crypto primitive
to use, it is very difficult to publish a useful single set of benchmarks;
instead, you are highly encouraged to run the benchmarks yourselves on your
target hardware.




Contributing
------------

The most important contributions are *uses* of *ring*. That is, we're very
interested in seeing useful things built on top of *ring*, like implementations
of TLS, SSH, the Noise Protocol, etc.

Of course, contributions to *ring*'s code base are highly appreciated too.
The *ring* project happily accepts pull requests without you needing to sign
any formal license agreement. The portions of pull requests that modify
existing files must be licensed under the same terms as the files being
modified. New files in pull requests, including in particular all Rust code,
must be licensed under the ISC-style license. Please state that you agree to
license your contributions in the commit messages of commits in pull requests,
e.g. by putting this at the bottom of your commit message:

```

I agree to license my contributions to each file under the terms given
at the top of each file I changed.
```


If
you want to work directly on *ring* and you don't have an idea for something to
contribute already, see these curated lists of open issues:

* [good-first-bug](https://github.com/briansmith/ring/labels/good-first-bug):
  Bugs that we think newcomers might find best to start with. Note that what
  makes a bug a good fit depends a lot on the developer's background and not
  just the hardness of the work.

In addition, we're always interested in these kinds of contributions:

* Expanded benchmarks in the
  [crypto-bench](https://github.com/briansmith/crypto-bench) project.
* Additional testing code and additional test vectors.
* Static analysis and fuzzing in the continuous integration.
* Support for more platforms in the continuous integration (e.g. Android, iOS,
  ARM microcontrollers).
* Documentation improvements.
* More code simplification, especially eliminating dead code.
* Improving the code size, execution speed, and/or memory footprint.
* Fixing any bugs you may have found.
* Better IDE support for Windows (e.g. running the tests within the IDE) and
  macOS (e.g. Xcode project files).

Before submitting pull requests, make sure that the tests succeed both when
running `cargo test` and `cargo test --no-default-features`. See
[BUILDING.md](BUILDING.md) for more info about the features flags that are
useful for people hacking on *ring*.



Versioning & Stability
----------------------

Users of *ring* should always use the latest released version, and users
should upgrade to the latest released version as soon as it is released.
*ring* has a linear release model that favors users of the latest released
version. We have never backported fixes to earlier releases and we don't
maintain branches other than the master branch. Further, for some obscure
technical reasons it's currently not possible to link two different versions
of *ring* into the same program; for policy reasons we don't bother to try
to work around that. Thus it is important that libraries using *ring* update
to the latest version of *ring* ASAP, so that libraries that depend on
*their* libraries can upgrade to the latest version of *ring*.

*ring* is tested on the latest Stable, Beta, and Nightly releases of Rust.
We do not spend effort on backward compatibility with older releases of
Rust; for example, when Rust 1.53 (Stable) is released, we don't care if
*ring* stops working with Rust 1.52 or earlier versions. Thus, we can
always use the latest *stable* features of the Rust language in *ring*.
So far we've never used unstable features of Rust except for the benchmarking
support (`#[bench]`), and we're hoping to remove even *that* Nightly
dependency. Sometimes things are broken with Nightly Rust. We prioritize
keeping things working on Stable; if things break on Beta and Nightly then
that breakage won't be considered urgent, though it will eventually get
resolved, one way or another.

We prefer to improve *ring*'s API over keeping *ring*'s API stable. We
don't keep old APIs around for the sake of backward compatibility; we prefer
to remove old APIs in the same change that adds new APIs. This makes it
easier for people to contribute improvements. This means that sometimes
upgrading to the newest version of *ring* will require some code changes. Over
time the rate of change in the API will probably slow to the point where it
will be stable in practice.

We don't have release notes. Instead, we try to clearly document each change
in each commit. Read the the commit message, the tests, and the patch itself
for each change. If anything is still unclear, let us know by submitting a pull
request or by filing an issue in the issue tracker so that we can improve
things.

This model of development is different than the model a lot of other open
source libraries use. The idea behind *our* model is to encourage all users to
work together to ensure that the latest version is good *as it is being
developed*. In particular, because users know that correctness/security fixes
(if any) aren't going to get backported, they have a strong incentive to help
review pull requests before they are merged and/or review commits on the master
branch after they've landed to ensure that code quality on the master branch
stays high.

The more common model, where there are stable versions that have important
security patches backported, lowers people's incentives to actively participate
in mainline development. Maintaining stable APIs also discourages improving
API design and internal code quality. Thus that model doesn't seem like a good
fit for *ring*.

Every six months we have a meeting to revisit this policy. Email
[brian@briansmith.org](mailto:brian@briansmith.org) if you want to attend
the next meeting. Please don't file issues regarding this policy.



Bug Reporting
-------------

Please report bugs either as pull requests or as issues in [the issue
tracker](https://github.com/briansmith/ring/issues). *ring* has a
**full disclosure** vulnerability policy. **Please do NOT attempt to report
any security vulnerability in this code privately to anybody.**



Online Automated Testing
------------------------

Travis CI is used for Android, Linux, and macOS. Appveyor is used for Windows.
The tests are run in debug and release configurations, for the current release
of each Rust channel (Stable, Beta, Nightly), for each configuration listed in
the table below. The C compilers listed are used for compiling the C portions.

<table>
<tr><th>OS</th><th>Arch.</th><th>Compilers</th><th>Status</th>
<tr><td rowspan=2>Linux</td>
    <td>x86, x86_64</td>
    <td>GCC 4.8, GCC 7, Clang 5</td>
    <td rowspan=4><a href=https://travis-ci.org/briansmith/ring/branches>Build Status</a></td>
</tr>
<tr><td>32&#8209;bit&nbsp;ARM, AAarch64</td>
    <td>GCC (Ubuntu/Linaro 4.8.4-2ubuntu1~14.04.1), tested using
        <code>qemu-user-arm</code>.</td>
</tr>
<tr><td>Android</td>
    <td>ARMv7, Aarch64</td>
    <td>*ring* for ARMv7 Android is built in CI using SDK version 26 targeting
        API level 18 (Android 4.3+); it is tested in the emulator using the
        corresponding system image. *ring* for AArch64 Android is built in CI
        using SDK version 26 targeting API level 21 (Android 5.0).</td>
</tr>
<tr><td>Mac&nbsp;OS&nbsp;X</td>
    <td>x64</td>
    <td>Apple LLVM version 9.0.0 (clang-900.0.39.2) from Xcode 9.3</td>
</tr>
<tr><td>Windows</td>
    <td>x86, x86_64</td>
    <td>MSVC 2015 Update 3 (14.0)</td>
    <td><a href=https://ci.appveyor.com/project/briansmith/ring/branch/master>Build Status</a></td>
</tr>
</table>



License
-------

See [LICENSE](LICENSE).
