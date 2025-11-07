from src.schemas import VerifyRequest, WmProfile


def test_wm_profile_roundtrip():
    data = {
        "tau_input": 0.9,
        "tau_feat": 0.1,
        "logit_band_low": -0.05,
        "logit_band_high": 0.05,
    }
    profile = WmProfile(**data)
    assert profile.tau_input == data["tau_input"]
    assert profile.tau_feat == data["tau_feat"]
    assert profile.logit_band_low == data["logit_band_low"]
    assert profile.logit_band_high == data["logit_band_high"]

    # Pydantic model -> dict -> model should preserve values.
    as_dict = profile.model_dump()
    profile2 = WmProfile(**as_dict)
    assert profile2 == profile


def test_verify_request_validation():
    req = VerifyRequest(
        aid="abcd" * 16,
        scheme_id="multi_factor_v1",
        evidence_hash="1234" * 16,
        wm_profile=WmProfile(
            tau_input=0.9,
            tau_feat=0.1,
            logit_band_low=-0.05,
            logit_band_high=0.05,
        ),
    )
    assert req.aid == "abcd" * 16
    assert req.scheme_id == "multi_factor_v1"
    assert req.evidence_hash == "1234" * 16
    assert isinstance(req.wm_profile, WmProfile)
