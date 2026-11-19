(* top_note = 8'bx *)
module top;
  parameter (* w = 1'bz *) P = 4'd3;
  localparam (* k = 'x *) Q = 4'd5;
  (* keep = 4'b1x01 *) reg [3:0] r;
  initial begin r = P + Q; $display("A P=%0d Q=%0d r=%0d", P, Q, r); $finish; end
endmodule
