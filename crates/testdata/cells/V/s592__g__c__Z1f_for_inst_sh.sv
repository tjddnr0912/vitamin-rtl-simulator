module sub #(parameter P = 0) (); initial #1 $display("@%m P=%0d", P); endmodule
module top;
  localparam N = 1;
  if (1) begin : gb
    for (genvar i = 0; i < N; i++) begin : L
      sub #(.P(10 + i)) u ();
    end
    localparam N = 3;
  end
  initial #5 $finish;
endmodule
