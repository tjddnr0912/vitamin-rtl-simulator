module sub #(parameter [64:0] P = 0);
  case (1)
    P: begin : a wire [7:0] w = 8'd1; initial #1 $display("NA12 a %m %0d", w); end
    default: begin : d wire [7:0] w = 8'd9; initial #1 $display("NA12 d %m %0d", w); end
  endcase
endmodule
module top;
  localparam [64:0] P = 65'd0;
  sub #(.P(65'd1)) ua[1:0] ();
endmodule
