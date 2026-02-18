from setuptools import setup, find_packages

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="memoryos-sdk",
    version="0.2.0",
    author="MemoryOS Team",
    author_email="team@memoryos.dev",
    description="Python SDK for MemoryOS - High-performance AI Agent memory management",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/BAI-LAB/MemoryOS",
    packages=find_packages(),
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: Apache Software License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
    ],
    python_requires=">=3.8",
    install_requires=[
        "requests>=2.28.0",
    ],
)
