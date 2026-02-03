from setuptools import setup

setup(
    name="binary-options-tools-uni",
    version="0.1.0",
    packages=["src"],
    license="MIT",
    description="",
    long_description="this is a test",
    long_description_content_type="text/markdown",
    install_requires=[],
    data_files=[("dlls", "dlls/binary_options_tools_uni.dll"), ("", ["version"])],
)
