from setuptools import setup

setup(
    name="ports-info",
    version="1.0.0",
    packages=["p2"],
    scripts=['bin/ports-info'],
    data_files=[
        ('share/applications', ['data/ports-info.desktop']),
        ('share/icons/hicolor/scalable/apps', ['ports-info.svg']),
    ],
    install_requires=[
        'PyGObject',
    ],
    author='mFat',
    author_email='newmfat@gmail.com',
    description='A GTK4 application for monitoring system ports',
    url='https://github.com/mfat/ports-info',
    license='GPL-3.0',
    classifiers=[
        'License :: OSI Approved :: GNU General Public License v3 (GPLv3)',
        'Programming Language :: Python :: 3',
        'Operating System :: POSIX :: Linux',
    ],
)