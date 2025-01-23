import React, { useState } from "react";
import Modal from "./Modal";

export function Card({
  children,
  className,
  title,
  modalContent,
}: {
  children: React.ReactNode;
  className?: string;
  title?: string;
  modalContent?: React.ReactNode;
}) {
  const [modalState, setModalState] = useState<{
    open: boolean;
    modal?: React.ReactElement;
  }>({ open: false, modal: undefined });

  const closeModal = () => {
    setModalState({
      open: false,
      modal: undefined,
    });
  };

  const openModal = () => {
    setModalState({
      open: true,
      modal: (
        <Modal onClose={closeModal} title={title || ""}>
          {modalContent}
        </Modal>
      ),
    });
  };

  return (
    <>
      {modalState.open && modalState.modal}
      <button
        className="w-[320px] h-[150px] rounded-lg border-2 drop-shadow-xl hover:bg-slate-100 transition"
        onClick={openModal}
      >
        <div className={className ? `p-4 ${className}` : "p-4"}>{children}</div>
      </button>
    </>
  );
}

export default Card;
