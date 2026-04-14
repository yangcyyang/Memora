import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { RoleTypeIntro } from "./RoleTypeIntro";
import { ImportGuide } from "./ImportGuide";

export type OnboardingStep = "intro" | "import-guide" | "import-file";

export function OnboardingFlow() {
  const navigate = useNavigate();
  const [step, setStep] = useState<OnboardingStep>("intro");

  const handleIntroNext = () => {
    setStep("import-guide");
  };

  const handleGuideComplete = () => {
    // Navigate to file import page
    navigate({ to: "/import" });
  };

  const handleSkipToCreate = () => {
    // Skip import, go directly to persona creation
    navigate({ to: "/create" });
  };

  const handleImportComplete = () => {
    // After file import, go to create
    navigate({ to: "/create" });
  };

  switch (step) {
    case "intro":
      return <RoleTypeIntro onNext={handleIntroNext} />;
    
    case "import-guide":
      return (
        <ImportGuide
          onComplete={handleGuideComplete}
          onSkip={handleSkipToCreate}
        />
      );
    
    case "import-file":
      // Handled by router, navigate to import page
      navigate({ to: "/import" });
      return null;
    
    default:
      return <RoleTypeIntro onNext={handleIntroNext} />;
  }
}

// Re-export individual components for flexibility
export { RoleTypeIntro } from "./RoleTypeIntro";
export { ImportGuide } from "./ImportGuide";
