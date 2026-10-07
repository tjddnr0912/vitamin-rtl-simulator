module top;
  bit [7:0] mem [int];                 // associative byte memory
  initial begin
    $readmemh("p.hex", mem);           // $readmemh into an associative array (IEEE 1800-2017 21.4.1)
    $display("b0=%h b1=%h n=%0d", mem[32'h10], mem[32'h11], mem.num());
    $finish;
  end
endmodule
