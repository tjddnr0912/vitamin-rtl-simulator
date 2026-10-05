module sub #(parameter logic signed [3:0] P = 0);
  case (P)
    4'b1111: begin : a wire [7:0] w = 8'd1; initial #1 $display("NA12S a %m %0d", w); end
    default: begin : d wire [7:0] w = 8'd9; initial #1 $display("NA12S d %m %0d", w); end
  endcase
endmodule
module top;
  localparam logic [3:0] P = 4'd0;
  sub #(.P(-1)) ua[1:0] ();
endmodule
