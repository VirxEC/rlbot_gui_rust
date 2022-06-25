import sys

from rlbot.utils.requirements_management import get_missing_packages, get_packages_needing_upgrade

# this python script exists because at this point we need Python to be configured in order to check reqs anyways
# also porting this to pure Rust was a downwards spiral of copious amounts of regex
requirements_file = tuple(arg for arg in sys.argv if "requirements_file" in arg)[0].split('=')[1]
requires_tkinter = "requires_tkinter" in sys.argv


def get_missing_python_packages():
    special_reqs = []
    if requires_tkinter:
        special_reqs.append('tkinter')

    # this script shouldn't need to be called at all if we're using a virtual environment!
    return get_missing_packages(requirements_file=requirements_file, special_reqs=special_reqs)


def get_python_packages_needing_upgrade():
    # this script shouldn't need to be called if there's no requirements file!
    return get_packages_needing_upgrade(requirements_file=requirements_file)

requirements = [r.line for r in get_missing_python_packages() + get_python_packages_needing_upgrade()]

if len(requirements) > 0:
    out = "[\"" + "\",\"".join(requirements) + "\"]"
else:
    out = "[]"

print(out, end="", flush=True)
