package pk; localparam int K = -1; endpackage
module top;
  import pk::K;
  for (genvar i = 0; i < 4; i++) begin : g
    if (1) begin : h
      localparam [1:0] i = 2'd3;
      case (i) 3: begin : a initial $display("@G%0d 3", g[0].h.i); end default: begin : d initial $display("@G def"); end endcase
    end
  end
  if (1) begin : s
    localparam logic [3:0] K = 4'hF;
    case (-1) K: begin : a initial $display("@K misread"); end default: begin : d initial $display("@K def"); end endcase
    case (K) 15: begin : b initial $display("@Ks 15"); end -1: begin : c initial $display("@Ks m1"); end default: begin : e initial $display("@Ks def"); end endcase
  end
endmodule
