interface ifc #(parameter logic [3:0] P = 4'hF) ();
  case (P)
    -1: begin : a initial $display("@%m a"); end
    15: begin : b initial $display("@%m b"); end
    default: begin : d initial $display("@%m d"); end
  endcase
endinterface
module mtw #(parameter logic [3:0] P = 4'hF) ();
  case (P)
    -1: begin : a initial $display("@%m a"); end
    15: begin : b initial $display("@%m b"); end
    default: begin : d initial $display("@%m d"); end
  endcase
endmodule
module top;
  ifc i1 ();
  ifc #(.P(4'h0)) i2 ();
  mtw m1 ();
  mtw #(.P(4'h0)) m2 ();
endmodule
