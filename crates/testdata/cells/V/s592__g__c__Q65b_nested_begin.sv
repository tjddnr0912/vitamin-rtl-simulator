module top;
  localparam N = 5;
  if (1) begin : gb
    begin
      localparam N = 1;
    end
    initial #1 $display("@N=%0d", N);
  end
  initial #5 $finish;
endmodule
