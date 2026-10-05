module top;
  localparam N = 3;
  for (genvar i = 0; i < N; i++) begin : L
    localparam W = i + 4;
    if (i == 1) begin : t wire [W-1:0] w = '1; initial #1 $display("@L%0d then bits=%0d w=%0d", i, $bits(w), w); end
    else begin : e wire [W-1:0] w = '0; initial #1 $display("@L%0d else bits=%0d w=%0d", i, $bits(w), w); end
    case (i)
      0: begin : c localparam V = 10; initial #1 $display("@L%0d c0 V=%0d", i, V); end
      default: begin : c localparam V = 20; initial #1 $display("@L%0d cd V=%0d", i, V); end
    endcase
  end
  initial #5 $finish;
endmodule
