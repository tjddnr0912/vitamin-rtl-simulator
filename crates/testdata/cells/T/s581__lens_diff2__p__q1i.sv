module top;
  localparam [64:0] W = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    if (1) begin : h
      localparam [64:0] i = 65'h1_0000_0000_0000_0005;
      wire [64:0] wi = i;
      initial #1 $display("@h i=%0d wi=%0d", i, wi);
      case (i)
        65'h1_0000_0000_0000_0005: begin : hw initial #2 $display("@h hw"); end
        default: begin : dd initial #2 $display("@h dd"); end
      endcase
    end
  end
  if (1) begin : b
    localparam W = 7;
    wire [64:0] ww = W;
    initial #1 $display("@b W=%0d ww=%0d", W, ww);
    case (W) 7: begin : b7 initial #2 $display("@b b7"); end default: begin : bd initial #2 $display("@b bd"); end endcase
  end
  initial #3 $display("@top W=%0d", W);
endmodule
