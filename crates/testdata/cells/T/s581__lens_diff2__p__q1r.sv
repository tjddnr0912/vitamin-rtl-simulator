package pe;
  typedef enum logic [64:0] {PE0 = 65'h1_0000_0000_0000_0009, PE1} pe_t;
endpackage
module top;
  import pe::*;
  typedef enum logic [64:0] {E0 = 65'h1_0000_0000_0000_0005, E1} e_t;
  localparam [64:0] QE = E1 + 65'd1;
  localparam [64:0] QP = PE1 + 65'd1;
  wire [64:0] we = E1;
  wire [64:0] wp = PE1;
  initial #1 $display("@ E0=%0d E1=%0d QE=%0d we=%0d PE0=%0d PE1=%0d QP=%0d wp=%0d", E0, E1, QE, we, PE0, PE1, QP, wp);
endmodule
