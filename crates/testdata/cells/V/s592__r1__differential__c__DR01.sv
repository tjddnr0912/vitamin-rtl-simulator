module tree #(parameter N = 4) (input [N-1:0] a, output y);
  if (N == 1) begin : leaf
    assign y = a[0];
  end else begin : node
    wire l, r;
    tree #(.N(N/2)) lt(.a(a[N/2-1:0]), .y(l));
    tree #(.N(N - N/2)) rt(.a(a[N-1:N/2]), .y(r));
    assign y = l ^ r;
  end
endmodule
module top;
  reg [4:0] a = 5'b10110;
  wire y;
  tree #(.N(5)) t(.a(a), .y(y));
  initial #1 $display("@y=%b", y);
  initial #10 $finish;
endmodule
