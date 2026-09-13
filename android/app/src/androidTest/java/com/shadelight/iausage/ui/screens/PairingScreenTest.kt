package com.shadelight.iausage.ui.screens

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTextInput
import com.shadelight.iausage.data.PairingInfo
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test

class PairingScreenTest {
    @get:Rule val compose = createComposeRule()

    @Test fun secretSurvivesRecompositionAndClearsForANewPairing() {
        val first = PairingInfo("192.168.1.10", 28741, "0123456789abcdef", "ABCD-EFGH", "0.2.7")
        var pairing by mutableStateOf(first)
        var recompositions by mutableIntStateOf(0)
        compose.setContent {
            recompositions
            PendingPairingScreen(pairing, loading = false, onPair = {}, onUseAnotherQr = {})
        }

        val secret = compose.onNodeWithTag("pairing-secret")
        secret.performTextInput("cuatro-palabras-123")
        compose.runOnIdle { recompositions++ }
        compose.waitForIdle()
        assertEquals(
            "cuatro-palabras-123",
            secret.fetchSemanticsNode().config[SemanticsProperties.EditableText].text,
        )

        compose.runOnIdle { pairing = first.copy(deviceId = "fedcba9876543210") }
        compose.waitForIdle()
        assertEquals("", secret.fetchSemanticsNode().config[SemanticsProperties.EditableText].text)
    }
}
