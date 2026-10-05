module top;
  localparam [3:0] P = 4'b1000;
  case (P) inside
    4'b1?00: begin : g1 initial $display("gen g1"); end
    default: begin : g2 initial $display("gen g2"); end
  endcase
  initial #10 $finish;
endmodule
