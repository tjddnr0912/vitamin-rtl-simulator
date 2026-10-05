`timescale 1ns/1ns
class K; function logic m(input logic [3:0] a); return a inside {4'b1?00}; endfunction endclass
module t;
  logic [3:0] q [$];
  initial begin
    K k; k = new;
    q.push_back(4'b1100);
    $display("method %b queue-elem %b", k.m(4'b1000), q[0] inside {4'b1?00});
    $finish;
  end
endmodule
