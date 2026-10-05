package pa; localparam P = 3; localparam N = 5; endpackage
module child #(parameter P = 1, parameter TAG = 0) ();
  import pa::*;
  localparam [64:0] Q = P + 65'd1;
  wire [64:0] w = P;
  initial #1 $display("@%0d P=%0d Q=%0d w=%0d N=%0d", TAG, P, Q, w, N);
endmodule
module hdr import pa::*; #(parameter [64:0] P = 65'h1_0000_0000_0000_0007) ();
  localparam [64:0] Q = P + 65'd1;
  wire [64:0] w = P;
  initial #1 $display("@h P=%0d Q=%0d w=%0d N=%0d", P, Q, w, N);
endmodule
module top #(parameter [64:0] P = 65'h1_0000_0000_0000_0009);
  import pa::*;
  localparam [64:0] Q = P + 65'd1;
  wire [64:0] w = P;
  child #(.P(65'h1_0000_0000_0000_000B), .TAG(1)) u1();
  child #(.TAG(2)) u2();
  hdr u3();
  initial #1 $display("@t P=%0d Q=%0d w=%0d N=%0d", P, Q, w, N);
endmodule
