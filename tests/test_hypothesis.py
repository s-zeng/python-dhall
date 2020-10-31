import dhall
import pytest
import string
from hypothesis import given, assume, settings, strategies as st

# min, max: RFC 7159
st_naturals = st.integers(min_value=0, max_value=(2 ** 53) - 1)
st_int = st.integers(min_value=-(2 ** 53) + 1, max_value=(2 ** 53) - 1)
st_floats = st.floats(min_value=-(2 ** 53) + 1, max_value=(2 ** 53) - 1)

# st.floats would be nice, but then we need pytest.approx, which doesn't work with eg. text
st_json = st.recursive(
    st.booleans() | st.text() | st.none() | st_int,  # | st_floats
    lambda children: st.lists(children) | st.dictionaries(st.text(), children),
)

keywords = ["as"]

st_passing_json = st.recursive(
    st.booleans() | st.text() | st_int,  # | st_floats
    lambda children: st.dictionaries(
        st.text(alphabet=string.ascii_letters).filter(lambda x: x not in keywords),
        children,
    ),
)


@given(st_floats)
def test_floats(xs):
    assert dhall.loads(dhall.dumps(xs)) == pytest.approx(xs)  # fails when abs=0.05


@given(st.text())
def test_text(xs):
    assert dhall.loads(dhall.dumps(xs)) == xs


@given(st.booleans())
def test_bool(xs):
    assert dhall.loads(dhall.dumps(xs)) == xs


@given(st.lists(st_naturals, min_size=1))
def test_list_naturals(lst):
    assert dhall.loads(dhall.dumps(lst)) == lst


@given(
    st.lists(st.floats(min_value=-(2 ** 53) + 1, max_value=(2 ** 53) - 1), min_size=1)
)
def test_list_floats(lst):
    assert dhall.loads(dhall.dumps(lst)) == pytest.approx(lst)


@given(st.lists(st.text(), min_size=1))
def test_list_text(lst):
    assert dhall.loads(dhall.dumps(lst)) == lst


@given(st_passing_json)
def test_json_obj(j_obj):
    assert dhall.loads(dhall.dumps(j_obj)) == j_obj
